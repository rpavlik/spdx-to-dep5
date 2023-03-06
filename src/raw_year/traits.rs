// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::types::{FourDigitYear, TwoDigitYear, YearExpr};

pub(crate) trait IsProper {
    /// Is this a proper range, with the beginning year less than or equal to the end year?
    ///
    /// If both years are two-digit, we assume the century is the same
    fn is_proper(&self) -> bool;
}

pub(crate) trait TryIsProper {
    /// If we are able to know, is this range proper?
    ///
    /// If both years are two-digit, we assume the century is the same
    fn try_is_proper(&self) -> Option<bool>;
}

pub(crate) trait IsSingleYear {
    /// Is this a "single year" range, with the begin and end year equal?
    ///
    /// If both years are two-digit, we assume the century is the same
    fn is_single_year(&self) -> bool;
}

pub(crate) trait SingleYearNormalizationOptions {
    /// Get whether we allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century, and, if this is used on a range, the two-digit begin
    /// is less than or equal to the two-digit end so we cannot infer that they span Y2K
    fn get_allow_century_guess(&self) -> bool;
}
pub(crate) trait SetSingleYearNormalizationOptions: SingleYearNormalizationOptions {
    /// Set whether we allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century, and, if this is used on a range, the two-digit begin
    /// is less than or equal to the two-digit end so we cannot infer that they span Y2K
    fn allow_century_guess(self, allow: bool) -> Self;
}

pub(crate) trait YearRangeNormalizationOptions: SingleYearNormalizationOptions {
    /// Get whether, if both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    fn get_allow_assuming_y2k_span(&self) -> bool;

    /// Get whether we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    fn get_allow_mixed_size_implied_century_rollover(&self) -> bool;
}

pub(crate) trait SetYearRangeNormalizationOptions:
    SetSingleYearNormalizationOptions + YearRangeNormalizationOptions
{
    /// Set whether, if both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    fn allow_assuming_y2k_span(self, allow: bool) -> Self;

    /// Set whether we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    fn allow_mixed_size_implied_century_rollover(self, allow: bool) -> Self;
}

pub(crate) trait RawYear {
    /// Get the century, which is 1 + the "most significant" two digits of the year, if known.
    #[must_use]
    fn try_century(&self) -> Option<u16>;

    /// Get the least-significant two digits of the year.
    #[must_use]
    fn two_digit(&self) -> TwoDigitYear;

    /// Get the year as a four-digit year, if it actually is one
    #[must_use]
    fn try_as_four_digit(&self) -> Option<FourDigitYear>;

    /// Using a simple heuristic if needed, get the year as a four-digit year.
    #[must_use]
    fn to_four_digit(&self) -> FourDigitYear;

    /// If this is a two digit year, use the provided century to make a 4 digit year
    #[must_use]
    fn to_four_digit_with_century_hint(&self, century: u16) -> FourDigitYear;

    /// Wrap in a generic YearExpr enum, if not already done
    fn to_year_expr(&self) -> YearExpr;

    /// Get the number wrapped deep inside
    fn into_inner(self) -> u16;
}

pub(crate) trait ConfigurableRawYear: RawYear {
    /// Try converting this year to a 4 digit years, with the provided options constraining the conversion
    fn try_to_four_digit(
        &self,
        options: impl SingleYearNormalizationOptions,
    ) -> Option<FourDigitYear>;
}

pub(crate) trait RawYearRange {
    /// Get the beginning year, as a generic YearExpr
    fn begin(&self) -> YearExpr;
    /// Get the ending year, as a generic YearExpr
    fn end(&self) -> YearExpr;

    /// Convert this range so that both begin and end are four digit years,
    /// making our best guess if required. Always succeeds but some guesses are dubious
    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear);
}

pub(crate) trait ConfigurableRawYearRange: RawYearRange {
    /// Try converting this range to a proper range of 4 digit years, with the provided options constraining the conversion
    fn try_to_four_digit_range(
        &self,
        options: impl YearRangeNormalizationOptions + Copy,
    ) -> Option<(FourDigitYear, FourDigitYear)>;
}

impl<T: RawYear, U: RawYear> TryIsProper for (T, U) {
    fn try_is_proper(&self) -> Option<bool> {
        let b = self.0.to_year_expr();
        let e = self.1.to_year_expr();

        match (b, e) {
            (YearExpr::TwoDigit(b), YearExpr::TwoDigit(e)) => Some((b, e).is_proper()),
            (YearExpr::TwoDigit(_), YearExpr::FourDigit(_)) => None,
            (YearExpr::FourDigit(_), YearExpr::TwoDigit(_)) => None,
            (YearExpr::FourDigit(b), YearExpr::FourDigit(e)) => Some((b, e).is_proper()),
        }
    }
}
