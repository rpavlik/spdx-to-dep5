// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::pin::Pin;

use futures::{AsyncBufRead, AsyncBufReadExt};

use crate::{key_value_parser::{KeyValuePair, ParsedLine, TEXT_CLOSE_TAG, TEXT_OPEN_TAG}, record::Record};


async fn read_line<R: AsyncBufRead>(mut reader: &mut Pin<Box<R>>) -> Option<String> {
    let mut s = String::new();
    let _ = AsyncBufReadExt::read_line(&mut reader, &mut s).await.ok()?;
    let s = s.trim_end();
    Some(s.to_string())
}

async fn read_parsed_line<R: AsyncBufRead>(reader: &mut Pin<Box<R>>) -> Option<ParsedLine> {
    let s = read_line(reader).await?;

    let parsed = ParsedLine::from(&s[..]);
    match parsed {
        ParsedLine::RecordDelimeter => Some(ParsedLine::RecordDelimeter),
        ParsedLine::ValueOnly(v) => Some(ParsedLine::ValueOnly(v)),
        ParsedLine::KVPair(pair) => {
            if pair.value.contains(TEXT_OPEN_TAG) && !pair.value.contains(TEXT_CLOSE_TAG) {
                let mut value_lines = vec![pair.value];

                while let Some(line) = read_line(reader).await {
                    let has_close_tag = line.contains(TEXT_CLOSE_TAG);
                    value_lines.push(line);
                    if has_close_tag {
                        break;
                    }
                }
                Some(ParsedLine::KVPair(KeyValuePair {
                    key: pair.key,
                    value: value_lines.join("\n"),
                }))
            } else {
                Some(ParsedLine::KVPair(pair))
            }
        }
    }
}

pub async fn get_record<R: AsyncBufRead>(reader: &mut Pin<Box<R>>) -> Option<Record> {
    let mut fields = Record::default();
    loop {
        match read_parsed_line(reader).await? {
            ParsedLine::RecordDelimeter => {
                return Some(fields);
            }
            ParsedLine::ValueOnly(v) => {
                println!("badline: {}", v);
                panic!("Found a value-only line");
            }
            ParsedLine::KVPair(pair) => {
                fields.push_field(pair);
            }
        }
    }
}
