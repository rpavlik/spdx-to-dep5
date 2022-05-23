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
    env_logger::init();
    // load SPDX file
    let filename = env::args().nth(1);
    let filename = filename.unwrap_or_else(|| "summary.spdx".to_string());
    eprintln!("Opening {}", filename);

    let file = std::fs::read_to_string(filename)?;
    let doc = spdx_from_tag_value(&file)?;

    // Filter SPDX
    let spdx_information = doc
        .file_information
        .into_iter()
        .filter(|f| f.copyright_text != "NONE");
    // Limit which file extensions
    // let extensions = [".c", ".cpp", ".h", ".hpp", ".py", ".md"];
    // let spdx_information =
    //     spdx_information.filter(|f| extensions.iter().any(|ext| f.file_name.ends_with(ext)));

    // Turn into tree, and identify uniformly-licensed subtrees
    let mut tree: CopyrightDataTree = spdx_information.collect();
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
