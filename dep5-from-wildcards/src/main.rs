// Copyright 2021-2024, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use clap::{crate_authors, crate_description, Parser};
use copyright_statements::{Copyright, YearRangeNormalization};
use glob::Pattern;
use itertools::Itertools;
use serde::Deserialize;
use spdx_rs::{
    models::{FileInformation, SpdxExpression},
    parsers::spdx_from_tag_value,
};
use spdx_to_dep5::{
    cleanup::cleanup_copyright_text,
    cli_help::omit_or_normalize_none,
    deb822::{control_file::Paragraphs, dep5::FilesParagraph},
    tree::{make_paragraphs, CopyrightDataTree},
};

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

    /// SPDX Input file
    #[arg(default_value = "summary.spdx")]
    spdx_input: String,

    /// input file with wildcards
    #[arg(default_value = "wildcards.toml")]
    toml_input: String,

    /// Omit files with no copyright data
    #[arg(short, long)]
    omit_no_copyright: bool,
}

/// Corresponds to a `[[wildcards]]` entry in the TOML file.
#[derive(Deserialize)]
struct RawWildcardEntry {
    patterns: Vec<String>,
    license: String,
    copyright: String,
    comment: Option<String>,
}

/// Corresponds to the entire TOML file.
#[derive(Deserialize)]
struct WildcardsFile {
    wildcards: Vec<RawWildcardEntry>,
}

/// This is the fully-processed version of `RawWildcardEntry`.
struct WildcardEntry {
    patterns: Vec<Pattern>,
    license: SpdxExpression,
    copyright: Copyright,
    comment: Option<String>,
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
    fn matches(&self, filename: &str, license: &SpdxExpression, copyright: &Copyright) -> bool {
        self.patterns.iter().any(|p| p.matches(filename))
            && *license == self.license
            && self.copyright.contains(copyright)
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
        let license = val.license.to_string().into();
        let copyright = val.copyright.to_string().into();
        FilesParagraph {
            files,
            license,
            copyright,
            comment: val.comment.map(|c| c.into()),
        }
    }
}

/// Turn the expressions in the file into a OR expression.
fn info_in_file_to_expression(license_info_in_file: &[SpdxExpression]) -> SpdxExpression {
    let s = license_info_in_file
        .iter()
        .unique()
        // .map(|e| format!("({})", e))
        .map(ToString::to_string)
        .sorted()
        .join(" OR ");
    if let Ok(e) = SpdxExpression::parse(&s) {
        e
    } else {
        // TODO this is a fallback in case of error
        license_info_in_file.first().cloned().unwrap_or_default()
    }
}

/// Compare a file's information against a collection of wildcards
fn matches_wildcards(
    options: YearRangeNormalization,
    wildcards: &[WildcardEntry],
    item: &FileInformation,
) -> bool {
    let license_to_match = item
        .concluded_license
        .as_ref()
        .and_then(|concluded| {
            if *concluded != SpdxExpression::default() {
                Some(concluded.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| info_in_file_to_expression(&item.license_information_in_file));

    let copyright_text = cleanup_copyright_text(&item.copyright_text).join("\n");
    let filename = item
        .file_name
        .strip_prefix("./")
        .unwrap_or_else(|| &item.file_name);

    let parsed_copyright = Copyright::try_parse(options, &copyright_text);

    if let Ok(copyright) = parsed_copyright {
        // eprintln!("{}: {} ; {}", filename, &license_to_match, &copyright);
        return wildcards
            .iter()
            .any(|elt| elt.matches(filename, &license_to_match, &copyright));
    }
    eprintln!("{}: parse copyright failed", filename);
    false
}

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let args = Args::parse();

    let opts = YearRangeNormalization {
        allow_century_guess: args.allow_century_guess,
        allow_assuming_y2k_span: args.allow_assuming_y2k_span,
        allow_mixed_size_implied_century_rollover: args.allow_mixed_size_implied_century_rollover,
    };

    // load SPDX file
    let filename = args.spdx_input;
    eprintln!("Opening {filename}");
    let file = std::fs::read_to_string(filename)?;
    let spdx_doc = spdx_from_tag_value(&file)?;

    // Omit or normalize the "NONE" text that REUSE tends to put into SPDX files.
    let spdx_information: Vec<_> =
        omit_or_normalize_none(spdx_doc.file_information, args.omit_no_copyright);

    // Load TOML file
    let wildcard_entries: Vec<WildcardEntry> = {
        let filename = args.toml_input;
        eprintln!("Opening {filename}");
        let file = std::fs::read_to_string(filename)?;

        let raw_config: WildcardsFile = toml::from_str(&file)?;
        let wildcard_entries: Result<Vec<WildcardEntry>, anyhow::Error> = raw_config
            .wildcards
            .into_iter()
            .map(|raw| WildcardEntry::try_parse(opts, raw))
            .collect();
        wildcard_entries?
    };

    // Turn entries that do not match the wildcard into tree, and identify uniformly-licensed subtrees
    let data_tree: CopyrightDataTree = spdx_information
        .into_iter()
        .filter(|fi| !matches_wildcards(opts, &wildcard_entries, fi))
        .collect();
    // data_tree.propagate_metadata();

    // These are the ones from TOML
    let explicit_paragraphs = wildcard_entries.into_iter().map(|w| {
        let para: FilesParagraph = w.into();
        para
    });

    // These are the ones we need to add for completeness, sorted.
    let additional_paragraphs = make_paragraphs(data_tree).flatten_to_strings().sorted();

    // Everybody turns into a string
    let paragraphs: Vec<String> = explicit_paragraphs
        .flatten_to_strings()
        .chain(additional_paragraphs)
        .collect_vec();

    println!("{}", paragraphs.join("\n\n"));
    Ok(())
}

#[test]
fn verify_cli() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
