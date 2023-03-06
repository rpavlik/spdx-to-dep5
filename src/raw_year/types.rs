// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::util;

pub(crate) trait IsProper {
    /// Is this a proper range, with the beginning year less than or equal to the end year?
    ///
    /// If both years are two-digit, we assume the century is the same
    fn is_proper(&self) -> bool;
}

trait TryIsProper {
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
trait SetSingleYearNormalizationOptions: SingleYearNormalizationOptions {
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

trait SetYearRangeNormalizationOptions:
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

#[derive(Debug, Clone, Copy, Default)]
struct YearRangeNormalization {
    /// Should allow the century to be guessed entirely when there is no four-digit year
    /// suitably close to imply a century?
    allow_century_guess: bool,
    /// If both years of a range are two-digit years, and the second is smaller than the first,
    /// can we assume the years span Y2K? This is a reasonable assumption as long as you are working
    /// with computer software in the 21st century.
    allow_assuming_y2k_span: bool,
    /// Should we allow the century part of a year range's endpoint to be inferred
    /// across a century boundary based on the other endpoint's known century.
    allow_mixed_size_implied_century_rollover: bool,
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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum YearExpr {
    TwoDigit(TwoDigitYear),
    FourDigit(FourDigitYear),
}

impl YearExpr {
    /// Make a two-digit year from an integer
    pub(crate) fn new_two_digit(y: u16) -> Self {
        YearExpr::TwoDigit(TwoDigitYear::new(y))
    }
    /// Make a four-digit year from an integer
    pub(crate) fn new_four_digit(y: u16) -> Self {
        YearExpr::FourDigit(FourDigitYear::new(y))
    }
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
        options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)>;
}

/// Newtype wrapping a "two digit year" - one that excludes the century and wraps every 100 years
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub(crate) struct TwoDigitYear(u16);

impl TwoDigitYear {
    /// Create a new two digit year
    pub(crate) fn new(year: u16) -> Self {
        assert!(year < 100);
        Self(year)
    }
}

impl RawYear for TwoDigitYear {
    fn try_century(&self) -> Option<u16> {
        None
    }

    fn two_digit(&self) -> TwoDigitYear {
        *self
    }

    fn try_as_four_digit(&self) -> Option<FourDigitYear> {
        None
    }

    fn to_four_digit(&self) -> FourDigitYear {
        FourDigitYear(util::guess_four_digit_from_two_digit(self.0))
    }

    fn to_four_digit_with_century_hint(&self, century: u16) -> FourDigitYear {
        FourDigitYear(util::compose_year(century, self.0))
    }

    fn to_year_expr(&self) -> YearExpr {
        YearExpr::TwoDigit(*self)
    }
    fn into_inner(self) -> u16 {
        self.0
    }
}

/// Newtype wrapping a "four digit year" - one that won't wrap after 99 years.
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Hash)]
pub(crate) struct FourDigitYear(u16);

impl FourDigitYear {
    /// Create a new four digit year
    pub(crate) fn new(year: u16) -> Self {
        assert!(year > 99);
        Self(year)
    }

    /// A four-digit year always can report the century, so this returns an integer unconditionally
    pub(crate) fn century(&self) -> u16 {
        util::get_century(self.0)
    }
}

impl RawYear for FourDigitYear {
    fn try_century(&self) -> Option<u16> {
        Some(self.century())
    }

    fn two_digit(&self) -> TwoDigitYear {
        TwoDigitYear(util::get_two_digit_year(self.0))
    }

    fn try_as_four_digit(&self) -> Option<FourDigitYear> {
        Some(*self)
    }

    fn to_four_digit(&self) -> FourDigitYear {
        *self
    }

    fn to_four_digit_with_century_hint(&self, _century: u16) -> FourDigitYear {
        *self
    }

    fn to_year_expr(&self) -> YearExpr {
        YearExpr::FourDigit(*self)
    }
    fn into_inner(self) -> u16 {
        self.0
    }
}

impl RawYear for YearExpr {
    fn try_century(&self) -> Option<u16> {
        match self {
            YearExpr::TwoDigit(y) => y.try_century(),
            YearExpr::FourDigit(y) => y.try_century(),
        }
    }

    fn two_digit(&self) -> TwoDigitYear {
        match self {
            YearExpr::TwoDigit(y) => y.two_digit(),
            YearExpr::FourDigit(y) => y.two_digit(),
        }
    }

    fn try_as_four_digit(&self) -> Option<FourDigitYear> {
        match self {
            YearExpr::TwoDigit(_) => None,
            YearExpr::FourDigit(y) => Some(*y),
        }
    }

    fn to_four_digit(&self) -> FourDigitYear {
        match self {
            YearExpr::TwoDigit(y) => y.to_four_digit(),
            YearExpr::FourDigit(y) => y.to_four_digit(),
        }
    }

    fn to_four_digit_with_century_hint(&self, century: u16) -> FourDigitYear {
        match self {
            YearExpr::TwoDigit(y) => y.to_four_digit_with_century_hint(century),
            YearExpr::FourDigit(y) => y.to_four_digit_with_century_hint(century),
        }
    }

    fn to_year_expr(&self) -> YearExpr {
        *self
    }

    fn into_inner(self) -> u16 {
        match self {
            YearExpr::TwoDigit(y) => y.into_inner(),
            YearExpr::FourDigit(y) => y.into_inner(),
        }
    }
}

// *******************************
// Handle ranges as pairs of years
// *******************************

/// (4-digit, 4-digit)
impl RawYearRange for (FourDigitYear, FourDigitYear) {
    fn begin(&self) -> YearExpr {
        self.0.to_year_expr()
    }

    fn end(&self) -> YearExpr {
        self.1.to_year_expr()
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        // we are already cool
        *self
    }
}

impl ConfigurableRawYearRange for (FourDigitYear, FourDigitYear) {
    fn try_to_four_digit_range(
        &self,
        _options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)> {
        // we are already cool
        Some(*self)
    }
}

impl IsProper for (FourDigitYear, FourDigitYear) {
    fn is_proper(&self) -> bool {
        self.0 <= self.1
    }
}

impl IsSingleYear for (FourDigitYear, FourDigitYear) {
    fn is_single_year(&self) -> bool {
        self.0 == self.1
    }
}

/// (2-digit, 2-digit)
impl RawYearRange for (TwoDigitYear, TwoDigitYear) {
    fn begin(&self) -> YearExpr {
        self.0.to_year_expr()
    }

    fn end(&self) -> YearExpr {
        self.1.to_year_expr()
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        let b = self.0;
        let e = self.1;
        if e < b {
            // range spans y2k
            (
                b.to_four_digit_with_century_hint(20),
                e.to_four_digit_with_century_hint(21),
            )
        } else {
            // guess the first year's century, re-use it for the second year
            let b = b.to_four_digit();
            let e = e.to_four_digit_with_century_hint(b.century());
            (b, e)
        }
    }
}

impl ConfigurableRawYearRange for (TwoDigitYear, TwoDigitYear) {
    fn try_to_four_digit_range(
        &self,
        options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)> {
        let b = self.0;
        let e = self.1;
        if b <= e {
            if options.get_allow_century_guess() {
                // guess the first year's century, re-use it for the second year
                let b = b.to_four_digit();
                let e = e.to_four_digit_with_century_hint(b.century());
                return Some((b, e));
            }
        } else {
            // range spans y2k?
            if options.get_allow_assuming_y2k_span() {
                return Some((
                    b.to_four_digit_with_century_hint(20),
                    e.to_four_digit_with_century_hint(21),
                ));
            }
        }
        None
    }
}

impl IsSingleYear for (TwoDigitYear, TwoDigitYear) {
    fn is_single_year(&self) -> bool {
        self.0 == self.1
    }
}

impl IsProper for (TwoDigitYear, TwoDigitYear) {
    fn is_proper(&self) -> bool {
        self.0 <= self.1
    }
}

/// (4-digit, 2-digit)
impl RawYearRange for (FourDigitYear, TwoDigitYear) {
    fn begin(&self) -> YearExpr {
        self.0.to_year_expr()
    }

    fn end(&self) -> YearExpr {
        self.1.to_year_expr()
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        let b = self.0;
        let e = self.1;
        if e < b.two_digit() {
            // range spans turn of the century
            let century = b.century();
            let e = e.to_four_digit_with_century_hint(century + 1);
            (b, e)
        } else {
            // Propagate first year's century
            let e = e.to_four_digit_with_century_hint(b.century());
            (b, e)
        }
    }
}

impl ConfigurableRawYearRange for (FourDigitYear, TwoDigitYear) {
    fn try_to_four_digit_range(
        &self,
        options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)> {
        let b = self.0;
        let e = self.1;
        if b.two_digit() <= e {
            // Propagate first year's century
            let e = e.to_four_digit_with_century_hint(b.century());
            return Some((b, e));
        } else {
            // range spans turn of the century?
            if options.get_allow_mixed_size_implied_century_rollover() {
                let century = b.century();
                return Some((b, e.to_four_digit_with_century_hint(century + 1)));
            }
        }
        None
    }
}

/// (2-digit, 4-digit)
/// "Weird flex but ok" - unusual format but we can make some meaningful guesses.
impl RawYearRange for (TwoDigitYear, FourDigitYear) {
    fn begin(&self) -> YearExpr {
        self.0.to_year_expr()
    }

    fn end(&self) -> YearExpr {
        self.1.to_year_expr()
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        let b = self.0;
        let e = self.1;

        if b <= e.two_digit() {
            // Propagate second year's century
            let b = b.to_four_digit_with_century_hint(e.century());
            (b, e)
        } else {
            // range spans turn of the century
            let century = e.century();
            let b = b.to_four_digit_with_century_hint(century - 1);
            (b, e)
        }
    }
}

impl ConfigurableRawYearRange for (TwoDigitYear, FourDigitYear) {
    fn try_to_four_digit_range(
        &self,
        options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)> {
        let b = self.0;
        let e = self.1;
        if b <= e.two_digit() {
            // Propagate second year's century - this is still weird.
            // TODO make this configurable?
            let b = b.to_four_digit_with_century_hint(e.century());
            return Some((b, e));
        } else {
            // range spans turn of the century?
            if options.get_allow_mixed_size_implied_century_rollover() {
                let century = e.century();
                return Some((b.to_four_digit_with_century_hint(century - 1), e));
            }
        }
        None
    }
}

/// (2 or 4 digit, 2 or 4 digit) with years wrapped in enum
/// Basically just have to unwrap the enum and dispatch again
impl RawYearRange for (YearExpr, YearExpr) {
    fn begin(&self) -> YearExpr {
        self.0
    }

    fn end(&self) -> YearExpr {
        self.1
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        match (self.0, self.1) {
            (YearExpr::TwoDigit(b), YearExpr::TwoDigit(e)) => (b, e).to_four_digit_range(),
            (YearExpr::TwoDigit(b), YearExpr::FourDigit(e)) => (b, e).to_four_digit_range(),
            (YearExpr::FourDigit(b), YearExpr::TwoDigit(e)) => (b, e).to_four_digit_range(),
            (YearExpr::FourDigit(b), YearExpr::FourDigit(e)) => (b, e).to_four_digit_range(),
        }
    }
}

impl ConfigurableRawYearRange for (YearExpr, YearExpr) {
    fn try_to_four_digit_range(
        &self,
        options: impl YearRangeNormalizationOptions,
    ) -> Option<(FourDigitYear, FourDigitYear)> {
        match (self.0, self.1) {
            (YearExpr::TwoDigit(b), YearExpr::TwoDigit(e)) => {
                (b, e).try_to_four_digit_range(options)
            }
            (YearExpr::TwoDigit(b), YearExpr::FourDigit(e)) => {
                (b, e).try_to_four_digit_range(options)
            }
            (YearExpr::FourDigit(b), YearExpr::TwoDigit(e)) => {
                (b, e).try_to_four_digit_range(options)
            }
            (YearExpr::FourDigit(b), YearExpr::FourDigit(e)) => {
                (b, e).try_to_four_digit_range(options)
            }
        }
    }
}

impl<T: RawYear, U: RawYear> TryIsProper for (T, U) {
    fn try_is_proper(&self) -> Option<bool> {
        let b = self.0.to_year_expr();
        let e = self.1.to_year_expr();

        match (b, e) {
            (YearExpr::TwoDigit(b), YearExpr::TwoDigit(e)) => Some((b, e).is_proper()),
            (YearExpr::TwoDigit(_b), YearExpr::FourDigit(_e)) => None,
            (YearExpr::FourDigit(_b), YearExpr::TwoDigit(_e)) => None,
            (YearExpr::FourDigit(b), YearExpr::FourDigit(e)) => Some((b, e).is_proper()),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::raw_year::types::{FourDigitYear, IsProper, TryIsProper, TwoDigitYear};

    use super::{RawYear, RawYearRange};

    #[test]
    fn to_four_digit_year() {
        assert_eq!(TwoDigitYear(59).to_four_digit().into_inner(), 2059);

        assert_eq!(FourDigitYear(2059).to_four_digit().into_inner(), 2059);
        assert_eq!(FourDigitYear(1959).to_four_digit().into_inner(), 1959);

        assert_eq!(TwoDigitYear(95).to_four_digit().into_inner(), 1995);

        assert_eq!(FourDigitYear(1995).to_four_digit().into_inner(), 1995);
        assert_eq!(FourDigitYear(2095).to_four_digit().into_inner(), 2095);
    }

    #[test]
    fn with_guessed_century() {
        {
            assert_eq!(
                TwoDigitYear(59)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                1959
            );

            assert_eq!(
                FourDigitYear(2059)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                2059
            );
            assert_eq!(
                FourDigitYear(1959)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                1959
            );

            assert_eq!(
                TwoDigitYear(95)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                1995
            );

            assert_eq!(
                FourDigitYear(1995)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                1995
            );
            assert_eq!(
                FourDigitYear(2095)
                    .to_four_digit_with_century_hint(20)
                    .into_inner(),
                2095
            );
        }
        {
            assert_eq!(
                TwoDigitYear(59)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                2059
            );

            assert_eq!(
                FourDigitYear(2059)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                2059
            );
            assert_eq!(
                FourDigitYear(1959)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                1959
            );

            assert_eq!(
                TwoDigitYear(95)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                2095
            );

            assert_eq!(
                FourDigitYear(1995)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                1995
            );
            assert_eq!(
                FourDigitYear(2095)
                    .to_four_digit_with_century_hint(21)
                    .into_inner(),
                2095
            );
        }
    }

    #[test]
    fn two_digit_year() {
        assert_eq!(TwoDigitYear(59).two_digit().into_inner(), 59);

        assert_eq!(FourDigitYear(2059).two_digit().into_inner(), 59);
        assert_eq!(FourDigitYear(1959).two_digit().into_inner(), 59);

        assert_eq!(TwoDigitYear(95).two_digit().into_inner(), 95);

        assert_eq!(FourDigitYear(1995).two_digit().into_inner(), 95);
        assert_eq!(FourDigitYear(2095).two_digit().into_inner(), 95);
    }

    #[test]
    fn century() {
        assert_eq!(TwoDigitYear(59).try_century(), None);

        assert_eq!(FourDigitYear(2059).try_century(), Some(21));
        assert_eq!(FourDigitYear(1959).try_century(), Some(20));

        assert_eq!(TwoDigitYear(95).try_century(), None);

        assert_eq!(FourDigitYear(1995).try_century(), Some(20));
        assert_eq!(FourDigitYear(2095).try_century(), Some(21));
    }

    #[test]
    fn year_ranges() {
        let y2059 = FourDigitYear(2059);
        let y59 = TwoDigitYear(59);
        let y1995 = FourDigitYear(1995);
        let y95 = TwoDigitYear(95);

        assert!(!(y2059, y1995).is_proper());
        assert!((y1995, y2059).is_proper());
        assert!((y1995, y59).try_is_proper().is_none());
        assert!((y95, FourDigitYear(1959)).try_is_proper().is_none());
        assert!((y1995, y95).try_is_proper().is_none());
        assert!(!(y95, y59).is_proper());
        assert!((y95, y95).is_proper());
        assert!((y59, y95).is_proper());
        assert!((y59, y59).is_proper());

        assert_eq!((y2059, y1995).to_four_digit_range(), (y2059, y1995));
        assert_eq!((y1995, y2059).to_four_digit_range(), (y1995, y2059));
        assert_eq!((y1995, y59).to_four_digit_range(), (y1995, y2059));
        assert_eq!((y1995, y95).to_four_digit_range(), (y1995, y1995));
        assert_eq!((y95, y59).to_four_digit_range(), (y1995, y2059));
        assert_eq!((y95, y95).to_four_digit_range(), (y1995, y1995));
        assert_eq!(
            (y59, y95).to_four_digit_range(),
            (y2059, FourDigitYear(2095))
        );

        assert_eq!(
            (y59, TwoDigitYear(39)).to_four_digit_range(),
            (FourDigitYear(1959), FourDigitYear(2039))
        );
        assert_eq!((y59, y59).to_four_digit_range(), (y2059, y2059));
    }
}
