// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::BufRead;

use key_value_parser::{policies::SPDXParsePolicy, KVParser, ParserOutput};
use spdx_to_dep5::builder::{BuilderError, SPDXBuilder};

fn main() -> Result<(), BuilderError> {
    let file = std::fs::File::open("summary.spdx").unwrap();
    let line_reader = std::io::BufReader::new(file).lines();

    let mut parser: KVParser<SPDXParsePolicy> = KVParser::default();
    let mut builder = SPDXBuilder::default();
    for result in line_reader {
        let line = result.unwrap();
        if let Some(field) = parser.process_line(&line).ok_or_else_err_on_keyless(|| {
            BuilderError::Message("Found line with no key".to_string())
        })? {
            builder.handle_field(&field)?;
        }
    }
    let doc = builder.try_into_result().unwrap();

    println!("stuff: {:?}", doc);
    Ok(())
}
