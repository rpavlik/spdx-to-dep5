// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::Display;

use itertools::Itertools;
use nom::Finish;

use crate::{copyright_parsing, raw_year::traits::YearRangeNormalizationOptions, years::YearSpec};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecomposedCopyright {
    pub years: Vec<YearSpec>,
    pub holder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Copyright {
    Decomposable(DecomposedCopyright),
    MultilineDecomposable(Vec<DecomposedCopyright>),
    Complex(String),
}

impl DecomposedCopyright {
    pub fn new(years: &[YearSpec], holder: &str) -> Self {
        Self {
            years: years.into(),
            holder: holder.trim().to_string(),
        }
    }
}

impl Copyright {
    pub fn try_parse(
        options: impl YearRangeNormalizationOptions + Copy,
        statement: &str,
    ) -> Result<Self, CopyrightDecompositionError> {
        let copyright = copyright_parsing::copyright_lines(options)(statement)
            .finish()
            .map(|(_leftover, parsed)| parsed)?;
        Ok(copyright)
    }
}

impl Display for DecomposedCopyright {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}, {}",
            self.years.iter().map(YearSpec::to_string).join(", "),
            self.holder
        )
    }
}

impl Display for Copyright {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Copyright::Decomposable(c) => c.fmt(f),
            Copyright::MultilineDecomposable(v) => {
                write!(
                    f,
                    "{}",
                    v.iter().map(DecomposedCopyright::to_string).join("\n")
                )
            }
            Copyright::Complex(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed decomposing copyright: {0}")]
pub struct CopyrightDecompositionError(String);

impl From<nom::error::Error<&str>> for CopyrightDecompositionError {
    fn from(value: nom::error::Error<&str>) -> Self {
        CopyrightDecompositionError(value.to_string())
    }
}
