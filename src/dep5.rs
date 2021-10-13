// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Syntax for writing Debian DEP5 machine-readable copyright files
//!
//! See <https://dep-team.pages.debian.net/deps/dep5>

use crate::control_file::{
    MultilineField, Paragraph, ParagraphAccumulator, SingleLineField,
    SingleLineOrMultilineEmptyFirstLineField,
};

#[derive(Debug, Clone)]
pub struct HeaderParagraph {
    pub format: SingleLineField,
    pub upstream_name: Option<SingleLineField>,
    pub upstream_contact: Option<SingleLineField>,
    pub source: Option<SingleLineField>,
    pub disclaimer: Option<SingleLineOrMultilineEmptyFirstLineField>,
    pub comment: Option<SingleLineOrMultilineEmptyFirstLineField>,
    pub license: Option<SingleLineOrMultilineEmptyFirstLineField>,
    pub copyright: Option<MultilineField>,
}

impl Default for HeaderParagraph {
    fn default() -> Self {
        Self {
            format: SingleLineField(
                "https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/".to_string(),
            ),
            upstream_name: None,
            upstream_contact: None,
            source: None,
            disclaimer: None,
            comment: None,
            license: None,
            copyright: None,
        }
    }
}

impl Paragraph for HeaderParagraph {
    fn try_to_string(&self) -> Result<Option<String>, crate::control_file::ControlFileError> {
        Ok(Some(
            ParagraphAccumulator::default()
                .write("Format", &self.format)?
                .write("Upstream-Name", &self.upstream_name)?
                .write("Upstream-Contact", &self.upstream_contact)?
                .write("Source", &self.source)?
                .write("Disclaimer", &self.disclaimer)?
                .write("Comment", &self.comment)?
                .write("License", &self.license)?
                .write("Copyright", &self.copyright)?
                .to_string(),
        ))
    }
}
