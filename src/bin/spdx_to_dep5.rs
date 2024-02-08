// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use clap::{crate_authors, crate_description, ArgGroup, Parser};
use itertools::Itertools;
use spdx_rs::{models::FileInformation, parsers::spdx_from_tag_value};
use spdx_to_dep5::{
    deb822::{
        control_file::{Paragraph, Paragraphs},
        dep5::HeaderParagraph,
    },
    tree::{make_paragraphs, CopyrightDataTree},
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

fn is_copyright_text_empty(fi: &FileInformation) -> bool {
    match &fi.copyright_text {
        None => true,
        Some(v) => v == "NONE",
    }
}

fn main() -> Result<(), spdx_rs::error::SpdxError> {
    env_logger::init();
    let args = Args::parse();

    // load SPDX file
    let filename = args.input;
    eprintln!("Opening {}", filename);

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    // Omit or normalize the "NONE" text that REUSE tends to put into SPDX files.
    let spdx_information: Vec<_> = if args.omit_no_copyright {
        doc.file_information
            .into_iter()
            .filter(|f| !is_copyright_text_empty(f))
            .collect()
    } else {
        doc.file_information
            .into_iter()
            .map(|f| {
                if is_copyright_text_empty(&f) {
                    let mut f = f;
                    f.copyright_text = None;
                    f
                } else {
                    f
                }
            })
            .collect()
    };

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
