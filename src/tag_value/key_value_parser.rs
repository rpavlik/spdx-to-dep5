use std::fmt::Debug;

use serde::de::value;

use crate::record::RecordError;

// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A key-value pair.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

pub const TEXT_OPEN_TAG: &str = &"<text>";
pub const TEXT_CLOSE_TAG: &str = &"</text>";
const DELIM: &str = &": ";

/// The result of parsing a single line as a key: value.
///
/// Does not handle any kind of multi-line values.
pub enum ParsedLine {
    RecordDelimeter,
    ValueOnly(String),
    KVPair(KeyValuePair),
}

impl ParsedLine {
    /// true if the KVPair variant
    pub fn is_kv_pair(&self) -> bool {
        match self {
            ParsedLine::KVPair(_) => true,
            ParsedLine::RecordDelimeter => false,
            ParsedLine::ValueOnly(_) => false,
        }
    }

    /// Turns the KVPair variant into Some(KeyValuePair) and everything else into None
    pub fn pair(self) -> Option<KeyValuePair> {
        match self {
            ParsedLine::KVPair(pair) => Some(pair),
            ParsedLine::RecordDelimeter => None,
            ParsedLine::ValueOnly(_) => None,
        }
    }
}

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
struct TrivialParsePolicy {}
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
struct SPDXParsePolicy {}
impl TagValueParsePolicy for SPDXParsePolicy {
    fn process_value<'a>(&self, _key: &str, value: &'a str) -> ProcessedValue<'a> {
        let trimmed_val = value.trim();
        let has_open = trimmed_val.starts_with(TEXT_OPEN_TAG);
        let has_close = trimmed_val.ends_with(TEXT_CLOSE_TAG);

        if has_open && has_close {
            let value = &trimmed_val[TEXT_OPEN_TAG.len()..trimmed_val.len() - TEXT_CLOSE_TAG.len()];
            ProcessedValue::CompleteValue(value)
        } else if has_open && !has_close {
            let value = &trimmed_val[TEXT_OPEN_TAG.len()..];
            ProcessedValue::StartOfMultiline(Some(value))
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
        if line.ends_with(TEXT_CLOSE_TAG) {
            let value = &line[..line.len() - TEXT_CLOSE_TAG.len()];
            ProcessedContinuationValue::FinishMultiline(Some(value))
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
pub struct KVParserLineOutput {
    pub pair: Option<KeyValuePair>,
    pub line_number: usize,
}

impl KVParserLineOutput {
    pub fn into_inner(self) -> Option<KeyValuePair> {
        self.pair
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
    pub fn process_line(&mut self, line: &str) -> Result<KVParserLineOutput, RecordError> {
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
        Ok(KVParserLineOutput {
            pair: maybe_return_pair,
            line_number: self.line_num,
        })
    }
}

impl<P: TagValueParsePolicy + Debug + Default> Default for KVParser<P> {
    fn default() -> Self {
        Self::new(P::default())
    }
}

#[cfg(test)]
mod test {

    use crate::tag_value::key_value_parser::SPDXParsePolicy;
    use crate::tag_value::key_value_parser::TagValueParsePolicy;
    use crate::tag_value::key_value_parser::TrivialParsePolicy;

    use super::KeyValuePair;

    use super::KVParser;

    #[test]
    fn basics() {
        fn test_parser<P: TagValueParsePolicy>(mut parser: KVParser<P>) {
            assert_eq!(
                parser
                    .process_line("key: value")
                    .unwrap()
                    .into_inner()
                    .unwrap(),
                KeyValuePair {
                    key: "key".to_string(),
                    value: "value".to_string(),
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
                .unwrap()
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
            .unwrap()
            .into_inner()
            .is_none());

        assert_eq!(
            parser
                .process_line("value</text>")
                .unwrap()
                .into_inner()
                .unwrap(),
            KeyValuePair {
                key: "key".to_string(),
                value: "value
value"
                    .to_string(),
            }
        );
    }
}
