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
use spdx_to_dep5::{
    async_functions::get_record,
    record::{Record, RecordError},
    try_parse_spdx_doc_from_records,
};

fn generate_records<R: AsyncBufRead>(reader: R) -> impl Stream<Item = Record> {
    // let parsed_lines = lines.map(|line| ParsedLine::from(line));
    // let group = parsed_lines.take_while(async move|pl| pl.is_kv_pair());
    // let parser = RecordParser::new(reader);
    // pin_mut!(parser);
    let mut reader = Box::pin(reader);
    async_stream::stream! {
    // async {
        while let Some(record) = get_record(&mut reader).await {
            yield record;
        }
    }
    // }
}

async fn async_main() -> Result<(), RecordError> {
    let file = File::open("summary.spdx")
        .await
        .map_err(|e| RecordError::Message(e.to_string()))?;
    let reader = io::BufReader::new(file);
    let stream = generate_records(reader);
    let records: Vec<_> = stream.collect().await;
    let doc = try_parse_spdx_doc_from_records(records.into_iter());
    println!("stuff: {:?}", doc);
    Ok(())
}
fn main() -> Result<(), RecordError> {
    println!("Hello, world!");
    futures::executor::block_on(async_main())?;
    Ok(())
}
