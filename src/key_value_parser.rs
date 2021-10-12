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

/// The result of parsing a key: value line.
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

#[derive(Debug)]
pub struct KVParser {
    state: State,
    line_num: usize,
    pending_key: String,
    value_lines: Vec<String>,
}

pub struct KVParserLineOutput {
    pub pair: Option<KeyValuePair>,
    pub line_number: usize,
}

impl KVParserLineOutput {
    fn have_pair(pair: KeyValuePair, line_number: usize) -> Self {
        Self {
            pair: Some(pair),
            line_number,
        }
    }
    fn no_pair(line_number: usize) -> Self {
        Self {
            pair: None,
            line_number,
        }
    }

    pub fn into_inner(self) -> Option<KeyValuePair> {
        self.pair
    }
}
impl KVParser {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn process_line(&mut self, line: &str) -> Result<KVParserLineOutput, RecordError> {
        self.line_num += 1;
        let (pair, next_state) = match &mut self.state {
            State::Ready => {
                match ParsedLine::from(line) {
                    ParsedLine::RecordDelimeter => (None, State::Ready),
                    ParsedLine::ValueOnly(v) => {
                        println!("badline: {}", v);
                        panic!("Found a value-only line");
                    }
                    ParsedLine::KVPair(pair) => {
                        let trimmed_val = pair.value.trim();
                        let has_open = trimmed_val.starts_with(TEXT_OPEN_TAG);
                        let has_close = trimmed_val.ends_with(TEXT_CLOSE_TAG);

                        if has_open && has_close {
                            let value = trimmed_val
                                [TEXT_OPEN_TAG.len()..trimmed_val.len() - TEXT_CLOSE_TAG.len()]
                                .to_string();
                            (
                                Some(KeyValuePair {
                                    key: pair.key,
                                    value,
                                }),
                                State::Ready,
                            )
                        } else if has_open && !has_close {
                            self.pending_key = pair.key;
                            self.value_lines = vec![trimmed_val[TEXT_OPEN_TAG.len()..].to_string()];
                            (None, State::AwaitingCloseText)
                        } else {
                            // just plain text
                            (Some(pair), State::Ready)
                        }
                    }
                }
            }
            State::AwaitingCloseText => {
                let line = line.trim_end();
                if line.ends_with(TEXT_CLOSE_TAG) {
                    self.value_lines
                        .push(line[..line.len() - TEXT_CLOSE_TAG.len()].to_string());
                    let value = self.value_lines.join("\n");
                    self.value_lines.clear();
                    let key = std::mem::take(&mut self.pending_key);
                    (Some(KeyValuePair { key, value }), State::Ready)
                } else {
                    self.value_lines.push(line.to_string());
                    (None, State::AwaitingCloseText)
                }
            }
        };
        self.state = next_state;
        Ok(KVParserLineOutput {
            pair,
            line_number: self.line_num,
        })
    }
}

impl Default for KVParser {
    fn default() -> Self {
        Self {
            state: State::Ready,
            line_num: 0,
            pending_key: String::new(),
            value_lines: vec![],
        }
    }
}

#[cfg(test)]
mod test {
    use crate::key_value_parser::KeyValuePair;

    use super::KVParser;

    #[test]
    fn basics() {
        let mut parser = KVParser::new();
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
    #[test]
    fn trim_same_line() {
        let mut parser = KVParser::new();
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
        let mut parser = KVParser::new();
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
