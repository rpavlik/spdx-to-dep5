// Copyright 2021-2025, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use clap::{crate_authors, crate_description, ArgGroup, Parser};
use itertools::Itertools;
use spdx_rs::{models::FileInformation, parsers::spdx_from_tag_value};
use spdx_to_dep5::{
    cli_help::omit_or_normalize_none,
    deb822::{
        control_file::{Paragraph, Paragraphs},
        dep5::HeaderParagraph,
    },
    tree::{make_paragraphs, CopyrightDataTree},
};

#[derive(Parser, Debug)]
#[command(author=crate_authors!(), version, about=crate_description!())]
#[command(group(
            ArgGroup::new("filter")
                .args(["include", "exclude"]),
        ))]
struct Args {
    /// Input file
    #[arg(default_value = "summary.spdx")]
    input: String,

    /// Extensions to exclude
    #[arg(short = 'x', long)]
    exclude: Vec<String>,

    /// The only extensions to include. Conflicts with --exclude.
    #[arg(short, long)]
    include: Vec<String>,

    /// Omit files with no copyright data
    #[arg(short, long)]
    omit_no_copyright: bool,

    /// Should allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century?
    #[arg(long)]
    allow_century_guess: bool,

    /// If both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    #[arg(long)]
    allow_assuming_y2k_span: bool,

    /// Should we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    #[arg(long)]
    allow_mixed_size_implied_century_rollover: bool,
}

/// Filter files according to arguments (at most one of `exclude` and `include` may be non-empty)
/// and collect into a `CopyrightDataTree` so the return value may be uniform (and because we need it anyway)
fn filter_files(
    iter: impl Iterator<Item = FileInformation>,
    exclude: Vec<String>,
    include: Vec<String>,
) -> CopyrightDataTree {
    if !exclude.is_empty() {
        iter.filter(|f| !exclude.iter().any(|ext| f.file_name.ends_with(ext)))
            .collect()
    } else if !include.is_empty() {
        iter.filter(|f| include.iter().any(|ext| f.file_name.ends_with(ext)))
            .collect()
    } else {
        iter.collect()
    }
}

fn main() -> Result<(), spdx_rs::error::SpdxError> {
    env_logger::init();
    let args = Args::parse();

    // load SPDX file
    let filename = args.input;
    eprintln!("Opening {filename}");

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    // Omit or normalize the "NONE" text that REUSE tends to put into SPDX files.
    let spdx_information: Vec<_> =
        omit_or_normalize_none(doc.file_information, args.omit_no_copyright);

    // Turn into tree, and identify uniformly-licensed subtrees
    let mut tree: CopyrightDataTree =
        filter_files(spdx_information.into_iter(), args.exclude, args.include);
    tree.propagate_metadata();

    // Turn into debian copyright file paragraphs
    let paragraphs: Vec<String> = HeaderParagraph::default()
        .try_to_string_ok()
        .into_iter()
        .chain(make_paragraphs(tree).flatten_to_strings().sorted())
        .collect();
    println!("{}", paragraphs.join("\n\n"));
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
