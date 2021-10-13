/*
 * Copyright 2021, Collabora, Ltd.
 *
 * SPDX-License-Identifier: Apache-2.0 OR MIT
 */

use std::{collections::HashMap, convert::TryInto, io::BufRead, ops::RangeBounds, pin::Pin};

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
    builder::SPDXBuilder,
    record::{Record, RecordError},
    tag_value::KVParser,
};

fn main() -> Result<(), RecordError> {
    println!("Hello, world!");
    // futures::executor::block_on(async_main())?;
    let file = std::fs::File::open("summary.spdx").unwrap();
    let mut line_reader = std::io::BufReader::new(file).lines();

    let mut parser = KVParser::new();
    let mut builder = SPDXBuilder::default();
    while let Some(result) = line_reader.next() {
        let line = result.unwrap();
        if let Some(field) = parser.process_line(&line)?.into_inner() {
            builder
                .handle_field(&field)
                .map_err(|e| RecordError::Message(e.to_string()))?;
        }
    }
    let doc = builder.try_into_result().unwrap();

    println!("stuff: {:?}", doc);
    Ok(())
}
