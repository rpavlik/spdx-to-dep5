// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::util;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum YearExpr {
    TwoDigit(TwoDigitYear),
    FourDigit(FourDigitYear),
}

impl YearExpr {
    fn two_digit(y: u16) -> Self {
        YearExpr::TwoDigit(TwoDigitYear(y))
    }
    fn four_digit(y: u16) -> Self {
        YearExpr::FourDigit(FourDigitYear(y))
    }
}

trait RawYear {
    #[must_use]
    fn try_century(&self) -> Option<u16>;

    #[must_use]
    fn two_digit(&self) -> TwoDigitYear;

    /// Using a simple heuristic if needed
    #[must_use]
    fn to_four_digit(&self) -> FourDigitYear;

    /// If this is a two digit year, use the provided century to make a 4 digit year
    #[must_use]
    fn with_guessed_century(&self, century: u16) -> FourDigitYear;

    fn to_year_expr(&self) -> YearExpr;

    fn into_inner(self) -> u16;
}

trait RawYearRange {
    fn begin(&self) -> YearExpr;
    fn end(&self) -> YearExpr;
    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear);

    /// Is this a proper year range? (the end year equal to or later than the beginning?)
    fn is_proper(&self) -> bool;
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct TwoDigitYear(u16);

impl RawYear for TwoDigitYear {
    fn try_century(&self) -> Option<u16> {
        None
    }

    fn two_digit(&self) -> TwoDigitYear {
        *self
    }

    fn to_four_digit(&self) -> FourDigitYear {
        FourDigitYear(util::guess_four_digit_from_two_digit(self.0))
    }

    fn with_guessed_century(&self, century: u16) -> FourDigitYear {
        FourDigitYear(util::compose_year(century, self.0))
    }

    fn to_year_expr(&self) -> YearExpr {
        YearExpr::TwoDigit(*self)
    }

    fn into_inner(self) -> u16 {
        self.0
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
struct FourDigitYear(u16);

impl FourDigitYear {
    fn century(&self) -> u16 {
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

    fn to_four_digit(&self) -> FourDigitYear {
        *self
    }

    fn with_guessed_century(&self, _century: u16) -> FourDigitYear {
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

    fn to_four_digit(&self) -> FourDigitYear {
        match self {
            YearExpr::TwoDigit(y) => y.to_four_digit(),
            YearExpr::FourDigit(y) => y.to_four_digit(),
        }
    }

    fn with_guessed_century(&self, century: u16) -> FourDigitYear {
        match self {
            YearExpr::TwoDigit(y) => y.with_guessed_century(century),
            YearExpr::FourDigit(y) => y.with_guessed_century(century),
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

    fn is_proper(&self) -> bool {
        self.0.into_inner() <= self.1.into_inner()
    }
}

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
        if e.0 < b.0 {
            // range spans y2k
            (b.with_guessed_century(19), e.with_guessed_century(20))
        } else {
            // guess the first year's century, re-use it for the second year
            let b = b.to_four_digit();
            let e = e.with_guessed_century(b.century());
            (b, e)
        }
    }

    fn is_proper(&self) -> bool {
        // we assume it always is proper...
        true
    }
}

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
        if e.two_digit().0 < b.two_digit().0 {
            // range spans turn of the century
            let century = b.century();
            let e = e.with_guessed_century(century + 1);
            (b, e)
        } else {
            // Propagate first year's century
            let e = e.with_guessed_century(b.century());
            (b, e)
        }
    }

    fn is_proper(&self) -> bool {
        // we assume it always is proper...
        true
    }
}

// Weird flex but ok
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
        if e.two_digit().0 < b.two_digit().0 {
            // range spans turn of the century
            let century = e.century();
            let b = b.with_guessed_century(century - 1);
            (b, e)
        } else {
            // Propagate second year's century
            let b = b.with_guessed_century(e.century());
            (b, e)
        }
    }
    fn is_proper(&self) -> bool {
        // we assume it always is proper...
        true
    }
}

impl RawYearRange for (YearExpr, TwoDigitYear) {
    fn begin(&self) -> YearExpr {
        self.0
    }

    fn end(&self) -> YearExpr {
        self.1.to_year_expr()
    }

    fn to_four_digit_range(&self) -> (FourDigitYear, FourDigitYear) {
        match self.0 {
            YearExpr::TwoDigit(b) => (b, self.1).to_four_digit_range(),
            YearExpr::FourDigit(b) => (b, self.1).to_four_digit_range(),
        }
    }
    fn is_proper(&self) -> bool {
        match self.0 {
            YearExpr::TwoDigit(b) => (b, self.1).is_proper(),
            YearExpr::FourDigit(b) => (b, self.1).is_proper(),
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::raw_year::types::{FourDigitYear, TwoDigitYear};

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
            assert_eq!(TwoDigitYear(59).with_guessed_century(19).into_inner(), 1959);

            assert_eq!(
                FourDigitYear(2059).with_guessed_century(19).into_inner(),
                2059
            );
            assert_eq!(
                FourDigitYear(1959).with_guessed_century(19).into_inner(),
                1959
            );

            assert_eq!(TwoDigitYear(95).with_guessed_century(19).into_inner(), 1995);

            assert_eq!(
                FourDigitYear(1995).with_guessed_century(19).into_inner(),
                1995
            );
            assert_eq!(
                FourDigitYear(2095).with_guessed_century(19).into_inner(),
                2095
            );
        }
        {
            assert_eq!(TwoDigitYear(59).with_guessed_century(20).into_inner(), 2059);

            assert_eq!(
                FourDigitYear(2059).with_guessed_century(20).into_inner(),
                2059
            );
            assert_eq!(
                FourDigitYear(1959).with_guessed_century(20).into_inner(),
                1959
            );

            assert_eq!(TwoDigitYear(95).with_guessed_century(20).into_inner(), 2095);

            assert_eq!(
                FourDigitYear(1995).with_guessed_century(20).into_inner(),
                1995
            );
            assert_eq!(
                FourDigitYear(2095).with_guessed_century(20).into_inner(),
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

        assert_eq!(FourDigitYear(2059).try_century(), Some(20));
        assert_eq!(FourDigitYear(1959).try_century(), Some(19));

        assert_eq!(TwoDigitYear(95).try_century(), None);

        assert_eq!(FourDigitYear(1995).try_century(), Some(19));
        assert_eq!(FourDigitYear(2095).try_century(), Some(20));
    }

    #[test]
    fn year_ranges() {
        let y2059 = FourDigitYear(2059);
        let y59 = TwoDigitYear(59);
        let y1995 = FourDigitYear(1995);
        let y95 = TwoDigitYear(95);

        assert!(!(y2059, y1995).is_proper());
        assert!((y1995, y2059).is_proper());
        assert!((y1995, y59).is_proper());
        assert!((y1995, y95).is_proper());
        assert!((y95, y59).is_proper());
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
