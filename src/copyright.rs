// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nom::Finish;

use crate::{copyright_parsing, raw_year::traits::YearRangeNormalizationOptions, years::YearSpec};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct DecomposedCopyright {
    years: Vec<YearSpec>,
    holder: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Copyright {
    Decomposable(DecomposedCopyright),
    MultilineDecomposable(Vec<DecomposedCopyright>),
    Complex(String),
}

impl DecomposedCopyright {
    pub(crate) fn new(years: &[YearSpec], holder: &str) -> Self {
        Self {
            years: years.into(),
            holder: holder.trim().to_string(),
        }
    }
}

impl Copyright {
    fn try_parse(
        options: impl YearRangeNormalizationOptions + Copy,
        statement: &str,
    ) -> Result<Self, nom::error::Error<&str>> {
        copyright_parsing::copyright_lines(options)(statement)
            .finish()
            .map(|(_leftover, parsed)| parsed)
    }
}
