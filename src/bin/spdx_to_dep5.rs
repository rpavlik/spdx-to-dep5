// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::{crate_authors, crate_description, Parser};
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
#[clap(author=crate_authors!(), version, about=crate_description!())]
struct Args {
    /// Input file
    #[clap(value_parser)]
    input: Option<String>,

    /// Extensions to exclude
    #[clap(short = 'x', long)]
    exclude: Vec<String>,

    /// The only extensions to include. Conflicts with --exclude.
    #[clap(short = 'i', long)]
    include: Vec<String>,
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

    if !args.exclude.is_empty() && !args.include.is_empty() {
        println!("Cannot specify both --include and --exclude!");
        panic!("Cannot specify both --include and --exclude");
    }

    // load SPDX file
    let filename = args.input.unwrap_or_else(|| "summary.spdx".to_string());
    eprintln!("Opening {}", filename);

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    // Filter SPDX
    let spdx_information = doc
        .file_information
        .into_iter()
        .filter(|f| f.copyright_text != "NONE");

    // Turn into tree, and identify uniformly-licensed subtrees
    let mut tree: CopyrightDataTree = filter_files(spdx_information, args.exclude, args.include);
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
