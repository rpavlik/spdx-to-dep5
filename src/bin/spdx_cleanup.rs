// Copyright 2021-2025, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use clap::{crate_authors, crate_description, Parser};

use copyright_statements::YearRangeNormalization;
use spdx_rs::{models::SPDX, parsers::spdx_from_tag_value};
use spdx_to_dep5::cli_help::omit_or_normalize_none;

#[derive(Parser, Debug)]
#[command(author=crate_authors!(), version, about=crate_description!())]
struct Args {
    /// Should allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century?
    #[arg(long = "allow-century-guess", action)]
    allow_century_guess: bool,

    /// If both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    #[arg(long, action)]
    allow_assuming_y2k_span: bool,

    /// Should we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    #[arg(long, action)]
    allow_mixed_size_implied_century_rollover: bool,

    /// Omit files with no copyright data
    #[arg(short, long)]
    omit_no_copyright: bool,

    /// Input file
    #[arg(default_value = "summary.spdx")]
    input: String,
}

fn main() -> Result<(), spdx_rs::error::SpdxError> {
    env_logger::init();
    let args = Args::parse();

    // load SPDX file
    let filename = args.input;
    eprintln!("Opening {filename}");

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    let _opts = YearRangeNormalization {
        allow_century_guess: args.allow_century_guess,
        allow_assuming_y2k_span: args.allow_assuming_y2k_span,
        allow_mixed_size_implied_century_rollover: args.allow_mixed_size_implied_century_rollover,
    };
    // Omit or normalize the "NONE" text that REUSE tends to put into SPDX files.
    let spdx_information: Vec<_> =
        omit_or_normalize_none(doc.file_information, args.omit_no_copyright);

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
