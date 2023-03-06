// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::traits::{
    SetSingleYearNormalizationOptions, SetYearRangeNormalizationOptions,
    SingleYearNormalizationOptions, YearRangeNormalizationOptions,
};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct YearRangeNormalization {
    /// Should allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century?
    pub(crate) allow_century_guess: bool,
    /// If both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    pub(crate) allow_assuming_y2k_span: bool,
    /// Should we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    pub(crate) allow_mixed_size_implied_century_rollover: bool,
}

impl YearRangeNormalization {
    pub(crate) fn new() -> Self {
        Default::default()
    }
}

impl SingleYearNormalizationOptions for YearRangeNormalization {
    fn get_allow_century_guess(&self) -> bool {
        self.allow_century_guess
    }
}

impl SetSingleYearNormalizationOptions for YearRangeNormalization {
    fn allow_century_guess(self, allow: bool) -> Self {
        Self {
            allow_century_guess: allow,
            ..self
        }
    }
}

impl YearRangeNormalizationOptions for YearRangeNormalization {
    fn get_allow_assuming_y2k_span(&self) -> bool {
        self.allow_assuming_y2k_span
    }

    fn get_allow_mixed_size_implied_century_rollover(&self) -> bool {
        self.allow_mixed_size_implied_century_rollover
    }
}

impl SetYearRangeNormalizationOptions for YearRangeNormalization {
    fn allow_assuming_y2k_span(self, allow: bool) -> Self {
        Self {
            allow_assuming_y2k_span: allow,
            ..self
        }
    }

    fn allow_mixed_size_implied_century_rollover(self, allow: bool) -> Self {
        Self {
            allow_mixed_size_implied_century_rollover: allow,
            ..self
        }
    }
}
