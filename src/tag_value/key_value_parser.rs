// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::Debug;

use super::parsed_line::{KeyValuePair, ParsedLine};

pub const TEXT_OPEN_TAG: &str = "<text>";
pub const TEXT_CLOSE_TAG: &str = "</text>";
const DELIM: &str = ": ";

/// Enum returned by the policy when processing a value.
pub enum ProcessedValue<'a> {
    /// Indicates that the provided value is complete and not continued on the following line.
    ///
    /// The data in this variant should have any multi-line decoration stripped.
    CompleteValue(&'a str),
    /// Indicates that the provided value is not complete, and that
    /// additional lines should be processed before terminating this key: value pair.
    /// If there is a value in this variant, it will be added as the first line in the overall pair value.
    ///
    /// The data in this variant should have any multi-line decoration stripped.
    StartOfMultiline(Option<&'a str>),
}

/// Enum returned by the policy when processing a continuation line for a multi-line value.
pub enum ProcessedContinuationValue<'a> {
    /// Indicates that the provided value is not complete, and that
    /// additional lines should be processed before terminating this key: value pair.
    /// If there is a value in this variant, it will be added as a line to the overall pair value.
    ///
    /// The data in this variant should have any multi-line decoration stripped.
    ContinueMultiline(Option<&'a str>),
    /// Indicates that the provided value terminates the multi-line value.
    /// If there is a value in this variant, it will be added as a line to the overall pair value.
    ///
    /// The data in this variant should have any multi-line decoration stripped.
    FinishMultiline(Option<&'a str>),
}

pub trait TagValueParsePolicy {
    fn process_value<'a>(&self, key: &str, value: &'a str) -> ProcessedValue<'a>;
    fn process_continuation<'a>(
        &self,
        key: &str,
        continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a>;
}

#[derive(Debug, Default, Clone, Copy)]
/// The simplest parse policy, that does no trimming or transformation, and no multi-line values.
pub struct TrivialParsePolicy {}
impl TagValueParsePolicy for TrivialParsePolicy {
    fn process_value<'a>(&self, _key: &str, value: &'a str) -> ProcessedValue<'a> {
        ProcessedValue::CompleteValue(value)
    }

    fn process_continuation<'a>(
        &self,
        _key: &str,
        _continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a> {
        unreachable!()
    }
}

#[derive(Debug, Default, Clone, Copy)]
/// The parse policy used for SPDX Tag-Value files, where a value that starts with `<text>` continues
/// possibly across multiple lines until `</text>`, both of which are trimmed.
pub struct SPDXParsePolicy {}
impl TagValueParsePolicy for SPDXParsePolicy {
    fn process_value<'a>(&self, _key: &str, value: &'a str) -> ProcessedValue<'a> {
        let trimmed_val = value.trim();
        if let Some(value) = trimmed_val.strip_prefix(TEXT_OPEN_TAG) {
            if let Some(value) = value.strip_suffix(TEXT_CLOSE_TAG) {
                // found both open and close
                ProcessedValue::CompleteValue(value)
            } else {
                // only found open
                ProcessedValue::StartOfMultiline(Some(value))
            }
        } else {
            // just plain text
            ProcessedValue::CompleteValue(value)
        }
    }

    fn process_continuation<'a>(
        &self,
        _key: &str,
        continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a> {
        let line = continuation_line.trim_end();
        if let Some(stripped) = line.strip_suffix(TEXT_CLOSE_TAG) {
            ProcessedContinuationValue::FinishMultiline(Some(stripped))
        } else {
            ProcessedContinuationValue::ContinueMultiline(Some(line))
        }
    }
}

impl From<&str> for ParsedLine {
    fn from(line: &str) -> Self {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            ParsedLine::RecordDelimeter
        } else {
            match line.match_indices(DELIM).next() {
                Some((delim, _)) => {
                    let (k, v) = line.split_at(delim);
                    let v = &v[DELIM.len()..];

                    ParsedLine::KVPair(KeyValuePair {
                        key: String::from(k),
                        value: String::from(v),
                    })
                }
                None => ParsedLine::ValueOnly(line.to_string()),
            }
        }
    }
}

#[derive(Debug, Clone)]
enum State {
    Ready,
    AwaitingCloseText,
}

/// The combination of possibly a key-value pair, plus the line number just processed
#[derive(Debug, Clone, PartialEq)]
pub struct KVParserLineOutput {
    pub pair: Option<KeyValuePair>,
    pub line_number: usize,
}

impl KVParserLineOutput {
    pub fn into_inner(self) -> Option<KeyValuePair> {
        self.pair
    }
    pub fn into_tuple(self) -> (Option<KeyValuePair>, usize) {
        (self.pair, self.line_number)
    }
}

/// A parser for key-value pairs (aka tag-value files).
///
/// Parameterized on handling of values to allow different
/// policies for e.g. handling multi-line values.
#[derive(Debug)]
pub struct KVParser<P> {
    policy: P,
    state: State,
    line_num: usize,
    pending_key: String,
    value_lines: Vec<String>,
}

impl<P: TagValueParsePolicy> KVParser<P> {
    pub fn new(policy: P) -> Self {
        Self {
            state: State::Ready,
            line_num: 0,
            pending_key: String::new(),
            value_lines: vec![],
            policy,
        }
    }
    fn maybe_push_value_line(&mut self, maybe_value: Option<&str>) {
        if let Some(value) = maybe_value {
            self.value_lines.push(value.to_string())
        }
    }
    pub fn process_line(&mut self, line: &str) -> KVParserLineOutput {
        self.line_num += 1;
        let (maybe_return_pair, next_state) = match &mut self.state {
            State::Ready => match ParsedLine::from(line) {
                ParsedLine::RecordDelimeter => (None, State::Ready),
                ParsedLine::ValueOnly(v) => {
                    println!("badline: {}", v);
                    panic!("Found a value-only line");
                }
                ParsedLine::KVPair(pair) => {
                    match self.policy.process_value(&pair.key, &pair.value) {
                        ProcessedValue::CompleteValue(value) => (
                            Some(KeyValuePair {
                                key: pair.key,
                                value: value.to_string(),
                            }),
                            State::Ready,
                        ),
                        ProcessedValue::StartOfMultiline(maybe_value) => {
                            self.pending_key = pair.key;
                            self.value_lines.clear();
                            self.maybe_push_value_line(maybe_value);
                            (None, State::AwaitingCloseText)
                        }
                    }
                }
            },
            State::AwaitingCloseText => {
                match self.policy.process_continuation(&self.pending_key, line) {
                    ProcessedContinuationValue::ContinueMultiline(maybe_value) => {
                        self.maybe_push_value_line(maybe_value);
                        (None, State::AwaitingCloseText)
                    }
                    ProcessedContinuationValue::FinishMultiline(maybe_value) => {
                        self.maybe_push_value_line(maybe_value);
                        let value = self.value_lines.join("\n");
                        self.value_lines.clear();
                        let key = std::mem::take(&mut self.pending_key);
                        (Some(KeyValuePair { key, value }), State::Ready)
                    }
                }
            }
        };
        self.state = next_state;
        KVParserLineOutput {
            pair: maybe_return_pair,
            line_number: self.line_num,
        }
    }
}

impl<P: TagValueParsePolicy + Debug + Default> Default for KVParser<P> {
    fn default() -> Self {
        Self::new(P::default())
    }
}

#[cfg(test)]
mod test {

    use crate::tag_value::key_value_parser::KVParserLineOutput;
    use crate::tag_value::key_value_parser::SPDXParsePolicy;
    use crate::tag_value::key_value_parser::TagValueParsePolicy;
    use crate::tag_value::key_value_parser::TrivialParsePolicy;

    use super::KeyValuePair;

    use super::KVParser;

    #[test]
    fn basics() {
        fn test_parser<P: TagValueParsePolicy>(mut parser: KVParser<P>) {
            assert_eq!(
                parser.process_line("key1: value1"),
                KVParserLineOutput {
                    pair: Some(KeyValuePair {
                        key: "key1".to_string(),
                        value: "value1".to_string(),
                    }),
                    line_number: 1
                }
            );
            assert_eq!(
                parser.process_line(" "),
                KVParserLineOutput {
                    pair: None,
                    line_number: 2
                }
            );
            assert_eq!(
                parser.process_line("key2: value2"),
                KVParserLineOutput {
                    pair: Some(KeyValuePair {
                        key: "key2".to_string(),
                        value: "value2".to_string(),
                    }),
                    line_number: 3
                }
            );
        }
        let parser: KVParser<TrivialParsePolicy> = KVParser::default();
        test_parser(parser);

        let parser: KVParser<SPDXParsePolicy> = KVParser::default();
        test_parser(parser);
    }

    #[test]
    fn trim_same_line() {
        let mut parser: KVParser<SPDXParsePolicy> = KVParser::default();
        assert_eq!(
            parser
                .process_line("key: <text>value</text>")
                .into_inner()
                .unwrap(),
            KeyValuePair {
                key: "key".to_string(),
                value: "value".to_string(),
            }
        );
    }

    #[test]
    fn long_value() {
        let mut parser: KVParser<SPDXParsePolicy> = KVParser::default();
        assert!(parser
            .process_line("key: <text>value")
            .into_inner()
            .is_none());

        assert_eq!(
            parser.process_line("value</text>").into_inner().unwrap(),
            KeyValuePair {
                key: "key".to_string(),
                value: "value
value"
                    .to_string(),
            }
        );
    }
}
