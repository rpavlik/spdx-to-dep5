// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::Debug;

use super::parsed_line::{KeyValuePair, ParsedLine};

/// Enum returned by a [TagValueParsePolicy] when processing a value.
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

/// Enum returned by a [TagValueParsePolicy] when processing a continuation line for a multi-line value.
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

/// Implement this policy to customize how [KVParser] works, mainly regarding multi-line values.
pub trait TagValueParsePolicy {
    /// Called when a key and value are parsed.
    ///
    /// Allows you to trim the value, as well as report
    /// that it is only the beginning of a multi-line value.
    fn process_value<'a>(&self, key: &str, value: &'a str) -> ProcessedValue<'a>;
    /// Called with each new line once [TagValueParsePolicy::process_value] returns
    /// [ProcessedValue::StartOfMultiline], to possibly trim or drop the line, and indicate
    /// when the multi-line value has finished.
    fn process_continuation<'a>(
        &self,
        key: &str,
        continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a>;
}

#[derive(Debug, Clone)]
enum State {
    Ready,
    AwaitingCloseText,
}

/// The output of processing a line of input in [KVParser]
#[derive(Debug, Clone, PartialEq)]
pub enum Output {
    /// The provided line was empty or whitespace-only
    EmptyLine,
    /// We are in the middle of a multi-line value
    ValuePending,
    /// The provided line had no key, but was not part of a multi-line value
    KeylessLine(String),
    /// The provided line was a value or completes a multi-line value
    Pair(KeyValuePair),
}

impl Default for Output {
    fn default() -> Self {
        Output::EmptyLine
    }
}

impl Output {
    fn compute_state(&self) -> State {
        match self {
            Output::EmptyLine => State::Ready,
            Output::ValuePending => State::AwaitingCloseText,
            Output::KeylessLine(_) => State::Ready,
            Output::Pair(_) => State::Ready,
        }
    }
}

/// The combination of possibly a key-value pair, plus the line number just processed
#[derive(Debug, Clone, PartialEq)]
pub struct OutputAndLine {
    pub output: Output,
    pub line_number: usize,
}

impl OutputAndLine {
    /// Extract the pair, or None if the output is something other than a pair.
    ///
    /// Similar to `Option<T>::ok()`
    pub fn pair(self) -> Option<KeyValuePair> {
        if let Output::Pair(pair) = self.output {
            Some(pair)
        } else {
            None
        }
    }
    /// Return an error if the output is a keyless line, otherwise extract the pair if present
    ///
    /// Similar to `Option<T>::ok_or()`
    pub fn pair_or_err_on_keyless<E>(self, err: E) -> Result<Option<KeyValuePair>, E> {
        match self.output {
            Output::EmptyLine => Ok(None),
            Output::ValuePending => Ok(None),
            Output::KeylessLine(_) => Err(err),
            Output::Pair(pair) => Ok(Some(pair)),
        }
    }
    /// Call a function that returns an error if the output is a keyless line, otherwise extract the pair if present.
    ///
    /// Similar to `Option<T>::ok_or_else()`
    pub fn pair_or_else_err_on_keyless<E, F: FnOnce() -> E>(
        self,
        err: F,
    ) -> Result<Option<KeyValuePair>, E> {
        match self.output {
            Output::EmptyLine => Ok(None),
            Output::ValuePending => Ok(None),
            Output::KeylessLine(_) => Err(err()),
            Output::Pair(pair) => Ok(Some(pair)),
        }
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
    pub fn process_line(&mut self, line: &str) -> OutputAndLine {
        self.line_num += 1;

        // Match on our current state to compute our output.
        //
        // The output also uniquely determines our next state.
        let output = match &mut self.state {
            State::Ready => match ParsedLine::from(line) {
                ParsedLine::RecordDelimeter => Output::EmptyLine,
                ParsedLine::ValueOnly(v) => Output::KeylessLine(v),
                ParsedLine::KVPair(pair) => {
                    match self.policy.process_value(&pair.key, &pair.value) {
                        ProcessedValue::CompleteValue(value) => Output::Pair(KeyValuePair {
                            key: pair.key,
                            value: value.to_string(),
                        }),
                        ProcessedValue::StartOfMultiline(maybe_value) => {
                            self.pending_key = pair.key;
                            self.value_lines.clear();
                            self.maybe_push_value_line(maybe_value);
                            Output::ValuePending
                        }
                    }
                }
            },
            State::AwaitingCloseText => {
                match self.policy.process_continuation(&self.pending_key, line) {
                    ProcessedContinuationValue::ContinueMultiline(maybe_value) => {
                        self.maybe_push_value_line(maybe_value);
                        Output::ValuePending
                    }
                    ProcessedContinuationValue::FinishMultiline(maybe_value) => {
                        self.maybe_push_value_line(maybe_value);
                        let value = self.value_lines.join("\n");
                        self.value_lines.clear();
                        let key = std::mem::take(&mut self.pending_key);
                        Output::Pair(KeyValuePair { key, value })
                    }
                }
            }
        };
        self.state = match &output {
            Output::EmptyLine => State::Ready,
            Output::ValuePending => State::AwaitingCloseText,
            Output::KeylessLine(_) => State::Ready,
            Output::Pair(_) => State::Ready,
        };
        OutputAndLine {
            output,
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

    use crate::tag_value::policies::SPDXParsePolicy;
    use crate::tag_value::policies::TrivialParsePolicy;

    use super::KVParser;
    use super::KeyValuePair;
    use super::Output;
    use super::OutputAndLine;
    use super::TagValueParsePolicy;

    #[test]
    fn basics() {
        fn test_parser<P: TagValueParsePolicy>(mut parser: KVParser<P>) {
            assert_eq!(
                parser.process_line("key1: value1"),
                OutputAndLine {
                    output: Output::Pair(KeyValuePair {
                        key: "key1".to_string(),
                        value: "value1".to_string(),
                    }),
                    line_number: 1
                }
            );
            assert_eq!(
                parser.process_line(" "),
                OutputAndLine {
                    output: Output::EmptyLine,
                    line_number: 2
                }
            );
            assert_eq!(
                parser.process_line("key2: value2"),
                OutputAndLine {
                    output: Output::Pair(KeyValuePair {
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
                .pair()
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
        assert!(parser.process_line("key: <text>value").pair().is_none());

        assert_eq!(parser.process_line("").output, Output::ValuePending);
        assert_eq!(
            parser.process_line("value</text>").pair().unwrap(),
            KeyValuePair {
                key: "key".to_string(),
                value: "value

value"
                    .to_string(),
            }
        );
        assert_eq!(parser.process_line("").output, Output::EmptyLine);
    }
}
