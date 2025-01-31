// Copyright 2021-2025, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::borrow::Borrow;
use std::convert::TryFrom;
use std::str::FromStr;

use copyright_statements::{Copyright, YearRangeNormalization};
use deb822_lossless::Deb822;
use glob::Pattern;
use itertools::Itertools;
use serde::Deserialize;
use spdx_rs::models::SpdxExpression;
use spdx_to_dep5::cleanup::StrExt;
use spdx_to_dep5::deb822::control_file::{
    ControlFileError, MultilineEmptyFirstLineField, Paragraph, ParagraphAccumulator,
    SingleLineField,
};
use spdx_to_dep5::deb822::dep5::FilesParagraph;

/// Corresponds to a `[[wildcards]]` entry in the TOML file.
#[derive(Deserialize)]
struct RawWildcardEntry {
    patterns: Vec<String>,
    license: String,
    copyright: String,
    comment: Option<String>,
}

#[derive(Deserialize)]
pub struct CopyrightFileIntro {
    format: String,
    upstream_name: String,
    source: String,
    files_excluded: Vec<String>,
    comment: Option<String>,
}

impl Paragraph for CopyrightFileIntro {
    fn try_to_string(&self) -> Result<Option<String>, ControlFileError> {
        Ok(Some(
            ParagraphAccumulator::default()
                .write("Format", &SingleLineField::from(self.format.clone()))?
                .write(
                    "Upstream-Name",
                    &SingleLineField::from(self.upstream_name.clone()),
                )?
                .write("Source", &SingleLineField::from(self.source.clone()))?
                .write(
                    "Comment",
                    &self
                        .comment
                        .as_ref()
                        .map(|c| SingleLineField::from(c.clone())),
                )?
                .write(
                    "Files-Excluded",
                    &MultilineEmptyFirstLineField::try_from(&self.files_excluded).ok(),
                )?
                .to_string(),
        ))
    }
}

#[derive(Deserialize)]
pub struct LicenseText {
    comment: Option<String>,
    license: String,
}

impl Paragraph for LicenseText {
    fn try_to_string(&self) -> Result<Option<String>, ControlFileError> {
        Ok(Some(
            ParagraphAccumulator::default()
                .write(
                    "Comment",
                    self.comment
                        .as_ref()
                        .map(|license| SingleLineField::from(license.clone()))
                        .borrow(),
                )?
                .write("License", &SingleLineField::from(self.license.clone()))?
                .to_string(),
        ))
    }
}

/// Corresponds to the entire TOML file.
#[derive(Deserialize)]
struct RawWildcardsFile {
    intro: Option<CopyrightFileIntro>,
    wildcards: Vec<RawWildcardEntry>,
    license_texts: Vec<LicenseText>,
}

/// This is the fully-processed version of `RawWildcardEntry`.
pub struct WildcardEntry {
    patterns: Vec<Pattern>,
    license: SpdxExpression,
    copyright: Copyright,
    comment: Option<String>,
}

pub struct ParsedData {
    pub intro: Option<CopyrightFileIntro>,
    pub exclude: Vec<Pattern>,
    pub wildcard_entries: Vec<WildcardEntry>,
    pub license_texts: Vec<LicenseText>,
}

impl WildcardEntry {
    /// Try to turn a `RawWildcardEntry` into a `WildcardEntry`
    fn try_parse(
        options: YearRangeNormalization,
        raw: RawWildcardEntry,
    ) -> Result<Self, anyhow::Error> {
        let wildcard: Vec<Pattern> = raw
            .patterns
            .iter()
            .map(|w| Pattern::new(w))
            .collect::<Result<Vec<_>, _>>()?;
        let license = SpdxExpression::parse(&raw.license)?;
        let copyright = Copyright::try_parse(options, &raw.copyright)?;
        Ok(WildcardEntry {
            patterns: wildcard,
            license,
            copyright,
            comment: raw.comment,
        })
    }

    /// Compare a `WildcardEntry` with the filename, license, and copyright data for a given file.
    /// Returns true if it matches.
    pub fn matches(&self, filename: &str, license: &SpdxExpression, copyright: &Copyright) -> bool {
        self.patterns.iter().any(|p| p.matches(filename))
            && *license == self.license
            && self.copyright.contains(copyright)
    }

    pub fn matches_wildcard(&self, filename: &str) -> bool {
        self.patterns.iter().any(|p| p.matches(filename))
    }

    pub fn matches_license_and_copyright(
        &self,
        license: &SpdxExpression,
        copyright: &Copyright,
    ) -> bool {
        *license == self.license && self.copyright.contains(copyright)
    }
}

/// Convert a `WildcardEntry` into a `FilesParagraph` to output for the `copyright` file
impl From<WildcardEntry> for FilesParagraph {
    fn from(val: WildcardEntry) -> Self {
        let files = val
            .patterns
            .iter()
            .map(ToString::to_string)
            .join("\n")
            .into();
        let license = val.license.to_string().licenses_spdx_to_debian().into();
        let copyright = val.copyright.to_string().into();
        FilesParagraph {
            files,
            license,
            copyright,
            comment: val.comment.map(|c| c.into()),
        }
    }
}

fn load_dep5(file: &str) -> Result<RawWildcardsFile, anyhow::Error> {
    let dep5 = Deb822::from_str(file)?;
    let intro: Option<CopyrightFileIntro> = dep5.paragraphs().next().and_then(|p| {
        let format = p.get("Format")?;
        let upstream = p.get("Upstream-Name")?;
        let source = p.get("Source")?;
        let comment = p.get("Comment");
        let excluded: Vec<String> = p
            .get("Files-Excluded")
            .map(|s| s.trim().split("\n").map(ToString::to_string).collect())
            .unwrap_or_default();
        Some(CopyrightFileIntro {
            format,
            upstream_name: upstream,
            source,
            comment,
            files_excluded: excluded,
        })
    });
    let patterns: Vec<RawWildcardEntry> = dep5
        .paragraphs()
        .filter_map(|p| {
            let files = p.get("Files")?;
            let license = p.get("License")?.licenses_debian_to_spdx();
            let copyright = p.get("Copyright")?;
            let comment = p.get("Comment");
            let patterns: Vec<String> = files
                .split('\n')
                .map(|line| line.trim().to_string())
                .collect();

            Some(RawWildcardEntry {
                patterns,
                license,
                copyright,
                comment,
            })
        })
        .collect();

    let licenses: Vec<LicenseText> = dep5
        .paragraphs()
        .filter_map(|p| {
            if p.contains_key("Files") {
                None
            } else {
                let license = p.get("License")?;
                let comment = p.get("Comment");
                Some(LicenseText { comment, license })
            }
        })
        .collect_vec();
    Ok(RawWildcardsFile {
        intro,
        wildcards: patterns,
        license_texts: licenses,
    })
}

/// Load a TOML or DEP5 (deb822 copyright) file, depending on extension.
pub fn load_config(
    filename: &str,
    opts: &YearRangeNormalization,
) -> Result<ParsedData, anyhow::Error> {
    eprintln!("Opening {filename}");
    let file = std::fs::read_to_string(filename)?;
    let raw: RawWildcardsFile = if filename.ends_with(".toml") {
        eprintln!("Parsing {filename} as TOML");
        toml::from_str(&file)?
    } else {
        eprintln!("Parsing {filename} as DEP5 (deb822 copyright)");
        load_dep5(&file)?
    };

    let exclude = raw
        .intro
        .as_ref()
        .map(|intro| {
            intro
                .files_excluded
                .iter()
                .filter_map(|p| Pattern::from_str(p).ok())
                .collect_vec()
        })
        .unwrap_or_default();

    // Now make them parsed wildcard entries.
    let wildcard_entries: Vec<WildcardEntry> = raw
        .wildcards
        .into_iter()
        .map(|raw| WildcardEntry::try_parse(*opts, raw))
        .collect::<Result<Vec<WildcardEntry>, _>>()?;
    Ok(ParsedData {
        intro: raw.intro,
        wildcard_entries,
        license_texts: raw.license_texts,
        exclude,
    })
}
