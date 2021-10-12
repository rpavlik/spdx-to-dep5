/*
 * Copyright 2021, Collabora, Ltd.
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{collections::HashMap, convert::TryInto, ops::RangeBounds, pin::Pin};

use async_std::{
    fs::{File, OpenOptions},
    io::{self},
    prelude::Stream,
    prelude::*,
};
use futures::{io::Lines, prelude::*, AsyncBufReadExt};
use futures::{pin_mut, StreamExt};
use spdx_rs::SPDX;
use spdx_to_dep5::record::Record;
use thiserror;


// struct Document {
//     version: String,
//     dataLicense: String,
//     SPDXID: String,
//     documentName: String,

// }


fn get_file_entry<R: AsyncBufRead>(record: Record) -> Option<Result<Record, RecordError>> {

    // {
    //     if let Some(filename) = record.value_for_key(KEY_FILENAME).unwrap_or(None) {
    //         let id = record.value_for_key(KEY_SPDXID)
    //         let entry = Entry {
    //             filename: filename.clone(),
    //             ,
    //         };
    //     }
    //     if record[0].key != KEY_FILENAME {
    //         continue;
    //     }
}
// fn lines(filename: &str) -> io::Result<io::Lines<io::BufReader<File>> {

//     let file = File::open(filename)?;
//     Ok(io::BufReader::new(file).lines()
// }
fn parse_entries<R: AsyncBufRead>(reader: R) -> impl Stream<Item = Result<Entry, RecordError>> {
    // let parsed_lines = lines.map(|line| ParsedLine::from(line));
    // let group = parsed_lines.take_while(async move|pl| pl.is_kv_pair());
    // let parser = RecordParser::new(reader);
    // pin_mut!(parser);
    let mut reader = Box::pin(reader);
    // async_stream::stream! {
    async {
        while let Some(record) = get_record(&mut reader).await {
            if let Some(filename) = record.value_for_key(KEY_FILENAME).unwrap_or(None) {
                let id = record.value_for_key(KEY_SPDXID)
                let entry = Entry {
                    filename: filename.clone(),
                    id: SpdxId(record.value_for_key(KEY_SPDXID)?),
                };
            }
            if record[0].key != KEY_FILENAME {
                continue;
            }
        }
    }
    // }
}


fn main() -> io::Result<()> {
    let file = File::open("summary.spdx")?;
    let lines = io::BufReader::new(file).lines();

    let spdx = SPDX::from_file("summary.spdx")?;
    spdx.get_files_for_package(package_spdx_id);
    println!("Hello, world!");
    Ok(())
}
