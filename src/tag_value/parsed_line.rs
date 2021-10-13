// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// A key-value pair.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

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
