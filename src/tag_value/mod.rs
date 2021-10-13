// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod key_value_parser;
pub mod parsed_line;

pub use key_value_parser::KVParser;

pub use parsed_line::{KeyValuePair, ParsedLine};
