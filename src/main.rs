/*
 * Copyright 2021, Collabora, Ltd.
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{collections::HashMap, ops::RangeBounds, pin::Pin};

use async_std::{
    fs::{File, OpenOptions},
    io::{self},
    prelude::Stream,
    prelude::*,
};
use futures::{io::Lines, prelude::*, AsyncBufReadExt};
use futures::{pin_mut, StreamExt};
use spdx_rs::SPDX;
use KeyValueParser::KeyValuePair;

use crate::KeyValueParser::ParsedLine;

mod KeyValueParser;

// struct Document {
//     version: String,
//     dataLicense: String,
//     SPDXID: String,
//     documentName: String,

// }

struct SpdxId(String);

enum Checksum {
    SHA1(String),
}
struct Entry {
    filename: String,
    id: SpdxId,
    fileChecksum: Checksum,
    licenseConcluded: Option<String>,
    licenseInfoInFile: String,
    fileCopyrightText: String,
}
const OPEN_TEXT: &str = &"<text>";
const CLOSE_TEXT: &str = &"</text>";

async fn line_not_contains_close_text(line: &String) -> bool {
    !line.contains(CLOSE_TEXT)
}

struct RecordParser<R> {
    reader: Pin<Box<R>>,
    // lines: Option<Lines<S>>,
    pending: Option<String>,
}
impl<R: AsyncBufRead + Unpin> RecordParser<R> {
    fn new(reader: R) -> RecordParser<R> {
        RecordParser {
            // lines: Some(AsyncBufReadExt::lines(reader))
            reader: Box::pin(reader),
            pending: None,
        }
    }

    async fn read_line(&mut self) -> Option<String> {
        let mut s = String::new();
        let _ = AsyncBufReadExt::read_line(&mut self.reader, &mut s)
            .await
            .ok()?;
        let s = s.trim_end();
        Some(s.to_string())
    }
    fn put_pending_line(mut self: Pin<&mut Self>, s: String) {
        self.pending = Some(s);
    }

    async fn get_parsed_line(&mut self) -> Option<ParsedLine> {
        let s = self.read_line().await?;

        let parsed = ParsedLine::from(s);
        match parsed {
            ParsedLine::RecordDelimeter => Some(ParsedLine::RecordDelimeter),
            ParsedLine::ValueOnly(v) => Some(ParsedLine::ValueOnly(v)),
            ParsedLine::KVPair(pair) => {
                if pair.value.contains(OPEN_TEXT) && !pair.value.contains(CLOSE_TEXT) {
                    let mut value_lines = vec![pair.value];

                    while let Some(line) = self.read_line().await {
                        let has_close_tag = line.contains(CLOSE_TEXT);
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
    async fn get_record(&mut self) -> Option<HashMap<String, String>> {
        let mut map: HashMap<String, String> = HashMap::new();
        loop {
            match self.get_parsed_line().await? {
                ParsedLine::RecordDelimeter => {
                    return Some(map);
                }
                ParsedLine::ValueOnly(_) => {
                    panic!("Found a value-only line");
                }
                ParsedLine::KVPair(pair) => {
                    map.insert(pair.key, pair.value);
                }
            }
        }
    }
}


fn parse_entries<R: AsyncBufRead>(reader: R) -> impl Stream<Item = Entry> {
    // let parsed_lines = lines.map(|line| ParsedLine::from(line));
    // let group = parsed_lines.take_while(async move|pl| pl.is_kv_pair());
    let parser = RecordParser::new(reader);
    pin_mut!(parser);
    async_stream::stream! {
        while let Some(record) = parser.get_record().await {

        }
    }
}

// fn lines(filename: &str) -> io::Result<io::Lines<io::BufReader<File>> {

//     let file = File::open(filename)?;
//     Ok(io::BufReader::new(file).lines()
// }

fn main() -> io::Result<()> {
    let file = File::open("summary.spdx")?;
    let lines = io::BufReader::new(file).lines();

    let spdx = SPDX::from_file("summary.spdx")?;
    spdx.get_files_for_package(package_spdx_id);
    println!("Hello, world!");
    Ok(())
}
