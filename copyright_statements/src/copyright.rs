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

impl DecomposedCopyright {
    fn contains(&self, other: &DecomposedCopyright) -> bool {
        self.holder.trim() == other.holder.trim()
            && other.years.iter().all(|other_spec| {
                // all of the other copyright's ranges must be included in some of our specs
                self.years
                    .iter()
                    .any(|self_spec| self_spec.contains(other_spec))
            })
    }
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
    pub fn new_from_single_yearspec(yearspec: &YearSpec, holder: &str) -> Self {
        Self {
            years: vec![yearspec.clone()],
            holder: holder.trim().to_string(),
        }
    }
}

fn vec_contains_decomposed(m: &[DecomposedCopyright], d2: &DecomposedCopyright) -> bool {
    m.iter().any(|d| d.contains(d2))
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

    pub fn contains(&self, other: &Copyright) -> bool {
        match self {
            Copyright::Decomposable(d) => match other {
                Copyright::Decomposable(d2) => d.contains(d2),
                Copyright::MultilineDecomposable(m2) => m2.iter().all(|d2| d.contains(d2)),
                Copyright::Complex(_) => false,
            },
            Copyright::MultilineDecomposable(m) => match other {
                Copyright::Decomposable(d2) => vec_contains_decomposed(m, d2),
                Copyright::MultilineDecomposable(m2) => {
                    m2.iter().all(|d2| vec_contains_decomposed(m, d2))
                }
                Copyright::Complex(_) => false,
            },
            Copyright::Complex(c) => match other {
                Copyright::Decomposable(_) => false,
                Copyright::MultilineDecomposable(_) => false,
                Copyright::Complex(c2) => c == c2,
            },
        }
    }

    #[cfg(test)]
    fn is_complex(&self) -> bool {
        matches!(self, Copyright::Complex(_))
    }

    #[cfg(test)]
    fn is_multiline_decomposable(&self) -> bool {
        matches!(self, Copyright::MultilineDecomposable(_))
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

#[derive(Debug, Clone, thiserror::Error)]
#[error("Failed decomposing copyright: {0}")]
pub struct CopyrightDecompositionError(String);

impl From<nom::error::Error<&str>> for CopyrightDecompositionError {
    fn from(value: nom::error::Error<&str>) -> Self {
        CopyrightDecompositionError(value.to_string())
    }
}

#[cfg(test)]
mod test {
    use crate::{Copyright, YearRangeNormalization};

    #[test]
    fn contains() {
        let two_liner = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2024, Rylie Pavlik
        Copyright 2020, 2022-2024, Collabora, Ltd.",
        )
        .unwrap();
        assert!(two_liner.is_multiline_decomposable());
        assert!(!two_liner.is_complex());

        let rylie_2024 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2024, Rylie Pavlik",
        )
        .unwrap();
        assert!(!rylie_2024.is_multiline_decomposable());
        assert!(!rylie_2024.is_complex());

        assert!(two_liner.contains(&rylie_2024));

        let collabora_year_and_range = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2020, 2022-2024, Collabora, Ltd.",
        )
        .unwrap();
        assert!(two_liner.contains(&collabora_year_and_range));

        let collabora_2020 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2020, Collabora, Ltd.",
        )
        .unwrap();
        assert!(two_liner.contains(&collabora_2020));

        let collabora_2021 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2021, Collabora, Ltd.",
        )
        .unwrap();
        assert!(!two_liner.contains(&collabora_2021));

        let collabora_2022_thru_2024 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2022-2024, Collabora, Ltd.",
        )
        .unwrap();
        assert!(two_liner.contains(&collabora_2022_thru_2024));

        let collabora_2022_thru_2023 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2022-2023, Collabora, Ltd.",
        )
        .unwrap();
        assert!(two_liner.contains(&collabora_2022_thru_2023));

        let collabora_2021_thru_2023 = Copyright::try_parse(
            YearRangeNormalization::default(),
            "Copyright 2021-2023, Collabora, Ltd.",
        )
        .unwrap();
        assert!(!two_liner.contains(&collabora_2021_thru_2023));
    }
}
