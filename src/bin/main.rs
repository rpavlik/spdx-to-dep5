// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use itertools::Itertools;
use key_value_parser::{policies::SPDXParsePolicy, KVParser, ParserOutput};
use lazy_static::lazy_static;
use regex::Regex;
use spdx_rs::models;
use spdx_to_dep5::{
    builder::{BuilderError, SPDXBuilder},
    control_file::Paragraph,
    dep5::FilesParagraph,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env,
    io::BufRead,
    path::{Path, PathBuf},
};

trait StrExt {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str;
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str>;
}
impl StrExt for str {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str {
        if let Some(prefix_removed) = self.strip_prefix(prefix) {
            prefix_removed.trim()
        } else {
            self
        }
    }
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str> {
        re.replace(self, "")
    }
}

fn cleanup_copyright_text(text: &str) -> Vec<Cow<str>> {
    lazy_static! {
        static ref RE: Regex = Regex::new("SPDX-License-Identifier:.*$").unwrap();
    }
    text.split("\n")
        .map(|line| {
            line.trim()
                .strip_prefix_if_present("SPDX-FileCopyrightText:")
                .strip_prefix_if_present("Copyright")
                .strip_prefix_if_present("(c)")
                .strip_prefix_if_present("(C)")
                .strip_match_if_present(&RE)
        })
        .sorted()
        .dedup()
        .collect()
}

/// A reference to both a parent directory and a set of all pathbufs in that directory, from [DirectoryAndFullPathBufMap]
struct FullPathBufsInADirectory<'a>(&'a Option<PathBuf>, &'a HashSet<PathBuf>);

/// A collection of full PathBuf paths, grouped by their parent directory
#[derive(Debug, Default)]
struct DirectoryAndFullPathBufMap(HashMap<Option<PathBuf>, HashSet<PathBuf>>);

impl DirectoryAndFullPathBufMap {
    fn insert_full_path(&mut self, filename: &str) -> bool {
        let filename = PathBuf::from(filename);
        let dir = filename.parent().map(|v| v.to_path_buf());
        self.0
            .entry(dir)
            .or_insert_with(|| HashSet::new())
            .insert(filename)
    }

    /// Iterate all full paths.
    fn iter_paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.0.values().flatten()
    }

    fn iter(&self) -> impl Iterator<Item = FullPathBufsInADirectory> {
        self.0.iter().map(|(k, v)| FullPathBufsInADirectory(k, v))
    }

    fn iter_dirs(&self) -> impl Iterator<Item = &Option<PathBuf>> {
        self.0.keys()
    }
}

// /// A collection of path references, grouped by their parent directory
// #[derive(Debug, Default)]
// struct DirectoryAndFullPathsMap<'a>(HashMap<Option<PathBuf>, HashSet<&'a Path>>);

// impl<'a> DirectoryAndFullPathsMap<'a> {
//     fn extend_from(mut self, collection: &'a DirectoryAndFullPathBufMap) -> Self {
//         for (dir, full_paths) in collection.0.iter() {
//             self.0
//                 .entry(dir.clone())
//                 .or_insert_with(|| HashSet::new())
//                 .extend(full_paths.iter().map(|p| Path::new(p)));
//         }

//         self
//     }

//     fn is_subset_of(&self, collection: &)
// }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FileGroupKey {
    copyright_text: String,
    license: String,
}

/// The file group keys associated with each parent directory
#[derive(Debug, Default)]
struct FileGroupKeysPerDirectory(HashMap<Option<PathBuf>, HashSet<FileGroupKey>>);

impl FileGroupKeysPerDirectory {
    fn insert(&mut self, fgk: FileGroupKey, dir: Option<PathBuf>) -> bool {
        self.0
            .entry(dir)
            .or_insert_with(|| HashSet::new())
            .insert(fgk)
    }

    fn extend_from<'a>(
        &'a mut self,
        fgk: &FileGroupKey,
        dirs: impl Iterator<Item = &'a Option<PathBuf>>,
    ) {
        for dir in dirs {
            self.insert(fgk.clone(), dir.clone());
        }
    }
}

struct FileKeyVal(FileGroupKey, DirectoryAndFullPathBufMap);

impl From<(FileGroupKey, DirectoryAndFullPathBufMap)> for FileKeyVal {
    fn from(v: (FileGroupKey, DirectoryAndFullPathBufMap)) -> Self {
        Self(v.0, v.1)
    }
}

#[derive(Debug, Default)]
struct AllFiles {
    entries: HashMap<FileGroupKey, DirectoryAndFullPathBufMap>,
}

impl AllFiles {
    // fn compute_summary_file_map(&self) -> DirectoryAndFullPathsMap {
    //     let mut ret = DirectoryAndFullPathsMap::default();
    //     for entry in self.entries.iter() {
    //         ret.extend_from(&entry.1);
    //     }
    //     ret
    // }

    fn compute_fgk_per_directory(&self) -> FileGroupKeysPerDirectory {
        let mut ret = FileGroupKeysPerDirectory::default();
        for (fgk, dirs_and_paths) in &self.entries {
            ret.extend_from(fgk, dirs_and_paths.iter_dirs())
        }
        ret
    }
}

impl FileKeyVal {
    fn into_files_paragraph(self) -> FilesParagraph {
        let (key, files) = (self.0, self.1);
        FilesParagraph {
            files: files
                .iter_paths()
                .map(|v| v.to_string_lossy())
                .collect_vec()
                .join("\n")
                .into(),
            copyright: key.copyright_text.into(),
            license: key.license.into(),
            comment: None,
        }
    }
}

impl AllFiles {
    fn from_iter(iter: impl Iterator<Item = models::FileInformation>) -> Self {
        let mut ret = Self::default();
        ret.accumulate_from_iter(iter);
        ret
    }
    fn into_paragraphs(self) -> impl Iterator<Item = FilesParagraph> {
        self.entries
            .into_iter()
            .map(|(key, files)| FileKeyVal(key, files).into_files_paragraph())
    }
    fn accumulate_from_iter(&mut self, iter: impl Iterator<Item = models::FileInformation>) {
        for item in iter {
            self.accumulate(&item);
        }
    }
    fn accumulate(&mut self, item: &models::FileInformation) {
        let license = item.license_information_in_file.join(" OR ");
        let copyright_text = cleanup_copyright_text(&item.copyright_text).join("\n");
        let filename = item.file_name.strip_prefix_if_present("./");
        let key = FileGroupKey {
            copyright_text,
            license,
        };
        self.entries
            .entry(key)
            .or_insert_with(|| DirectoryAndFullPathBufMap::default())
            .insert_full_path(filename);
    }
}

fn main() -> Result<(), BuilderError> {
    let filename = env::args().skip(1).next();
    let filename = filename.unwrap_or("summary.spdx".to_string());
    println!("Opening {}", filename);
    let file = std::fs::File::open(filename).unwrap();
    let line_reader = std::io::BufReader::new(file).lines();

    let mut parser: KVParser<SPDXParsePolicy> = KVParser::default();
    let mut builder = SPDXBuilder::default();
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
                        builder.handle_field(&field)?;
                    }
                }
            }
            Err(e) => {
                let bad_line_num = parser.process_line("").line_number();
                eprintln!("Got error {} on line {}", e, bad_line_num);
                return Err(BuilderError::Message(e.to_string()));
            }
        }
    }
    let doc = builder.try_into_result().unwrap();

    let spdx_information = doc
        .file_information
        .into_iter()
        .filter(|f| f.copyright_text != "NONE");
    let paragraphs: Vec<String> = AllFiles::from_iter(spdx_information)
        .into_paragraphs()
        .filter_map(|paragraph| paragraph.try_to_string().ok().flatten())
        .collect();
    println!("stuff: {}", paragraphs.join("\n\n"));
    Ok(())
}
