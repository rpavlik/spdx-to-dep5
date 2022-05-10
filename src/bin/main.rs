// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use itertools::Itertools;
use spdx_rs::parsers::spdx_from_tag_value;
use spdx_to_dep5::{
    deb822::{
        control_file::{Paragraph, Paragraphs},
        dep5::HeaderParagraph,
    },
    tree::{make_paragraphs, CopyrightDataTree},
};
use std::{
    collections::{HashMap, HashSet},
    env,
    path::PathBuf,
};

/// A collection of full PathBuf paths, grouped by their parent directory
#[derive(Debug, Default)]
struct DirectoryAndFullPathBufMap(HashMap<Option<PathBuf>, HashSet<PathBuf>>);

fn main() -> Result<(), spdx_rs::error::SpdxError> {
    // load SPDX file
    let filename = env::args().nth(1);
    let filename = filename.unwrap_or_else(|| "summary.spdx".to_string());
    eprintln!("Opening {}", filename);

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    // Filter SPDX
    let extensions = [".c", ".cpp", ".h", ".hpp", ".py", ".md"];
    let spdx_information = doc
        .file_information
        .into_iter()
        .filter(|f| f.copyright_text != "NONE")
        .filter(|f| extensions.iter().any(|ext| f.file_name.ends_with(ext)));

    // Turn into tree, and identify uniformly-licensed subtrees
    let mut tree = CopyrightDataTree::from_iter(spdx_information.clone());
    tree.propagate_metadata();

    // Turn into debian copyriht file paragraphs
    let paragraphs: Vec<String> = HeaderParagraph::default()
        .try_to_string_ok()
        .into_iter()
        .chain(make_paragraphs(tree).flatten_to_strings().sorted())
        .collect();
    println!("{}", paragraphs.join("\n\n"));
    Ok(())
}
