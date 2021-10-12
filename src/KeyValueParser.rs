use std::pin::Pin;

use futures::Stream;

/*
 * Copyright 2021, Collabora, Ltd.
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

pub struct KeyValuePair {
    pub key: String,
    pub value: String,
}

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

const DELIM: &str = &": ";

impl From<String> for ParsedLine {
    fn from(line: String) -> Self {
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
                None => ParsedLine::ValueOnly(line),
            }
        }
    }
}
