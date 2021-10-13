// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::BufRead;

use key_value_parser::{policies::SPDXParsePolicy, KVParser, ParserOutput};
use spdx_to_dep5::{
    builder::{BuilderError, SPDXBuilder},
    control_file::{ControlFileError, Paragraph},
    dep5::FilesParagraph,
};

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

    const NEEDLE: &str = "SPDX-FileCopyrightText: ";
    let paragraphs: Vec<String> = doc
        .file_information
        .into_iter()
        .filter(|f| f.copyright_text != "NONE")
        .map(|f| {
            if f.copyright_text.contains(NEEDLE) {
                let copr = f.copyright_text.replace(NEEDLE, "");
                let mut f = f;
                f.copyright_text = copr;
                f
            } else {
                f
            }
        })
        .map(|f| FilesParagraph {
            files: f.file_name.into(),
            copyright: f.copyright_text.into(),
            license: f.license_information_in_file.join("\n").into(), //f.concluded_license,
            comment: None,
        })
        .filter_map(|paragraph| paragraph.try_to_string().ok().flatten())
        .collect();
    println!("stuff: {}", paragraphs.join("\n\n"));
    Ok(())
}
