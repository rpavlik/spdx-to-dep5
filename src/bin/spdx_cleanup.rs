// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use clap::{crate_authors, crate_description, Parser};

use copyright_statements::{Copyright, YearRangeNormalization};
use spdx_rs::{
    models::{FileInformation, SPDX},
    parsers::spdx_from_tag_value,
};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// A collection of full PathBuf paths, grouped by their parent directory
#[derive(Debug, Default)]
struct DirectoryAndFullPathBufMap(HashMap<Option<PathBuf>, HashSet<PathBuf>>);

#[derive(Parser, Debug)]
#[command(author=crate_authors!(), version, about=crate_description!())]
struct Args {
    /// Input file
    #[arg(default_value = "summary.spdx")]
    input: String,

    /// Should allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century?
    #[arg(default_value = "false")]
    allow_century_guess: bool,

    /// If both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    #[arg(default_value = "false")]
    allow_assuming_y2k_span: bool,

    /// Should we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    #[arg(default_value = "false")]
    allow_mixed_size_implied_century_rollover: bool,
}

fn main() -> Result<(), spdx_rs::error::SpdxError> {
    env_logger::init();
    let args = Args::parse();

    // load SPDX file
    let filename = args.input;
    eprintln!("Opening {filename}");

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    let opts = YearRangeNormalization {
        allow_century_guess: args.allow_century_guess,
        allow_assuming_y2k_span: args.allow_assuming_y2k_span,
        allow_mixed_size_implied_century_rollover: args.allow_mixed_size_implied_century_rollover,
    };
    // Omit or normalize the "NONE" text that REUSE tends to put into SPDX files.
    let spdx_information: Vec<_> = doc
        .file_information
        .into_iter()
        .map(|f| {
            if f.copyright_text == "NONE" {
                let mut f = f;
                f.copyright_text = "NOASSERTION".to_string();
                f
            } else {
                f
            }
        })
        .map(|f| match Copyright::try_parse(opts, &f.copyright_text) {
            Ok(copyright) => FileInformation {
                copyright_text: copyright.to_string(),
                ..f
            },
            Err(_) => f,
        })
        .collect();
    let _doc = SPDX {
        file_information: spdx_information,
        ..doc
    };

    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
