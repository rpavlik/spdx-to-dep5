// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use itertools::Itertools;
use key_value_parser::{policies::SPDXParsePolicy, KVParser, ParserOutput};
use lazy_static::lazy_static;
use regex::Regex;
use spdx_rs::models;
use spdx_to_dep5::{builder::{BuilderError, SPDXBuilder}, control_file::{Paragraph, Paragraphs}, dep5::{FilesParagraph, HeaderParagraph}};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env,
    io::BufRead,
    iter,
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

    /// Get all paths, flattening directories to wildcards where possible
    fn iter_paths_concise<'a>(
        &'a self,
        fgk: &'a FileGroupKey,
        fgk_per_dir: &'a FileGroupKeysPerDirectory,
    ) -> impl Iterator<Item = String> + 'a {
        self.0
            .iter()
            .map(move |(dir, all_files)| fgk_per_dir.iter_collapsed_paths_for(dir, fgk, all_files))
            .flatten()
    }
    fn iter_dirs(&self) -> impl Iterator<Item = &Option<PathBuf>> {
        self.0.keys()
    }
}

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

    /// Look up a directory, and see if the provided FileGroupKey is the only one listed for the directory,
    /// meaning it's safe to wildcard.
    fn dir_has_only_this_fgk(&self, dir: &Option<PathBuf>, fgk: &FileGroupKey) -> bool {
        self.0.get(dir).map_or(false, |fgks_for_this_dir| {
            assert!(fgks_for_this_dir.contains(fgk));
            fgks_for_this_dir.len() == 1
        })
    }

    /// Collapse this list into a wildcard if possible.
    fn iter_collapsed_paths_for<'a>(
        &self,
        dir: &Option<PathBuf>,
        fgk: &FileGroupKey,
        files: &'a HashSet<PathBuf>,
    ) -> Box<dyn Iterator<Item = String> + 'a> {
        if self.dir_has_only_this_fgk(dir, fgk) {
            Box::new(iter::once(dir.as_ref().map_or("*".to_string(), |d| {
                d.join("*").to_string_lossy().to_string()
            })))
        } else {
            Box::new(files.iter().map(|x| x.to_string_lossy().to_string()))
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
    fn compute_fgk_per_directory(&self) -> FileGroupKeysPerDirectory {
        let mut ret = FileGroupKeysPerDirectory::default();
        for (fgk, dirs_and_paths) in &self.entries {
            ret.extend_from(fgk, dirs_and_paths.iter_dirs())
        }
        ret
    }
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
    fn into_concise_paragraphs(self) -> impl Iterator<Item = FilesParagraph> {
        let fgk_per_directory = self.compute_fgk_per_directory();
        self.entries.into_iter().map(move |(key, files)| {
            FileKeyVal(key, files).into_concise_files_paragraph(&fgk_per_directory)
        })
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

    fn into_concise_files_paragraph(
        self,
        fgk_per_dir: &FileGroupKeysPerDirectory,
    ) -> FilesParagraph {
        let (fgk, files) = (self.0, self.1);
        let files = files
            .iter_paths_concise(&fgk, fgk_per_dir)
            .sorted_unstable()
            .collect_vec()
            .join("\n");
        FilesParagraph {
            files: files.into(),
            copyright: fgk.copyright_text.into(),
            license: fgk.license.into(),
            comment: None,
        }
    }
}

fn main() -> Result<(), BuilderError> {
    let filename = env::args().skip(1).next();
    let filename = filename.unwrap_or("summary.spdx".to_string());
    eprintln!("Opening {}", filename);
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
    let paragraphs: Vec<String> = HeaderParagraph::default()
        .try_to_string_ok()
        .into_iter()
        .chain(
            AllFiles::from_iter(spdx_information)
                .into_concise_paragraphs()
                .flatten_to_strings()
                .sorted(),
        )
        .collect();
    println!("{}", paragraphs.join("\n\n"));
    Ok(())
}
