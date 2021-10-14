// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use key_value_parser::{policies::SPDXParsePolicy, KVParser, LineNumber};
use spdx_rs::models;

use crate::builder::{BuilderError, SPDXBuilder};

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{} (line {})", .0.value(), .0.line_number())]
    BuilderError(LineNumber<BuilderError>),
    #[error("Keyless line on line {0}")]
    KeylessLine(usize),
    #[error("Error reading: {0}")]
    ReadError(String),
}

impl From<ParseError> for BuilderError {
    fn from(v: ParseError) -> Self {
        BuilderError::Message(v.to_string())
    }
}

pub fn parse_tag_value<E: std::fmt::Display, I: Iterator<Item = Result<String, E>>>(
    line_reader: I,
) -> Result<(models::SPDX, Option<Vec<ParseError>>), ParseError> {
    let mut parser: KVParser<SPDXParsePolicy> = KVParser::default();
    let mut builder = SPDXBuilder::default();
    let mut errors = vec![];
    for result in line_reader {
        match result {
            Ok(line) => {
                let parse_result = parser.process_line(&line);
                let line_num = parse_result.line_number();
                match parse_result.into_inner() {
                    key_value_parser::Output::EmptyLine => {}
                    key_value_parser::Output::Pending => {}
                    key_value_parser::Output::KeylessLine(_) => {
                        eprintln!("Found keyless line on line {}", line_num);
                    }
                    key_value_parser::Output::Output(field) => {
                        if let Err(e) = builder.handle_field(&field) {
                            errors.push(ParseError::BuilderError(LineNumber::new(line_num, e)));
                        }
                    }
                }
            }
            Err(e) => {
                let bad_line_num = parser.process_line("").line_number();
                eprintln!("Got error {} on line {}", e, bad_line_num);
                return Err(ParseError::ReadError(e.to_string()));
            }
        }
    }
    let doc = builder.try_into_result().unwrap();
    Ok((
        doc,
        if errors.is_empty() {
            None
        } else {
            Some(errors)
        },
    ))
}
