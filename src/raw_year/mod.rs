// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

pub(crate) mod options;
pub(crate) mod parse;
pub(crate) mod traits;
mod types;
mod util;

/// Number of years in a century
pub(crate) const CENTURY_DURATION: u16 = 100;

pub(crate) use traits::{RawYear, RawYearRange};

pub use options::YearRangeNormalization;
