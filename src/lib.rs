// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub mod async_functions;
pub mod entry;
pub mod key_value_parser;
pub mod record;
pub mod builder;

pub use entry::try_parse_spdx_doc_from_records;
