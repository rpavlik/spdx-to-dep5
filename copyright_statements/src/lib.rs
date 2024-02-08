// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod copyright;
mod copyright_parsing;
pub mod raw_year;
mod years;

pub use copyright::{Copyright, CopyrightDecompositionError, DecomposedCopyright};
pub use raw_year::{
    options::YearRangeNormalization,
    traits::{SingleYearNormalizationOptions, YearRangeNormalizationOptions},
};
pub use years::{coalesce_years, Year, YearRange, YearRangeCollection, YearSpec};
