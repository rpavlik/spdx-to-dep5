// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::BufRead;

use spdx_to_dep5::{
    builder::SPDXBuilder, record::RecordError, tag_value::key_value_parser, tag_value::KVParser,
};

fn main() -> Result<(), RecordError> {
    let file = std::fs::File::open("summary.spdx").unwrap();
    let line_reader = std::io::BufReader::new(file).lines();

    let mut parser: KVParser<key_value_parser::SPDXParsePolicy> = KVParser::default();
    let mut builder = SPDXBuilder::default();
    for result in line_reader {
        let line = result.unwrap();
        if let Some(field) = parser
            .process_line(&line)
            .pair_or_err_on_keyless(RecordError::Message("Found line with no key".to_string()))?
        {
            builder
                .handle_field(&field)
                .map_err(|e| RecordError::Message(e.to_string()))?;
        }
    }
    let doc = builder.try_into_result().unwrap();

    println!("stuff: {:?}", doc);
    Ok(())
}
