// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fmt::Display;

use itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(pub u16);

impl Display for Year {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearRange {
    begin: Year,
    end: Year,
}

impl YearRange {
    pub(crate) fn new(begin: Year, end: Year) -> Self {
        assert!(begin <= end);
        Self { begin, end }
    }

    fn is_single_year(&self) -> bool {
        self.begin == self.end
    }

    fn can_add(&self, new_year: &Year) -> bool {
        // within the range
        (new_year <= &self.end && new_year >= &self.begin)
            || (*new_year == Year(self.end.0 + 1))// appends one year to the end
            || (*new_year == Year(self.begin.0 - 1)) // appends one year to the beginning
    }

    fn can_merge(&self, new_range: &YearRange) -> bool {
        self.can_add(&new_range.begin) || self.can_add(&new_range.end)
    }

    fn merge_with(self, other: YearRange) -> Self {
        Self::new(self.begin.min(other.begin), self.end.max(other.end))
    }

    fn partial_order_single_year(&self, single: &Year) -> Option<std::cmp::Ordering> {
        if self.is_single_year() {
            self.begin.partial_cmp(single)
        } else if self.begin == *single {
            // in normal partial order land we'll call this undefined
            None
        } else {
            // otherwise just compare on starting year
            Some(self.begin.cmp(single))
        }
    }

    fn order_single_year_for_merging(&self, single: &Year) -> std::cmp::Ordering {
        // make the range "smaller" so it sorts first
        self.partial_order_single_year(single)
            .unwrap_or(std::cmp::Ordering::Greater)
    }

    fn try_add(&self, new_year: Year) -> Option<Self> {
        if new_year <= self.end && new_year >= self.begin {
            Some(*self)
        } else if new_year == Year(self.end.0 + 1) {
            Some(Self {
                begin: self.begin,
                end: new_year,
            })
        } else if new_year == Year(self.begin.0 - 1) {
            Some(Self {
                begin: new_year,
                end: self.end,
            })
        } else {
            None
        }
    }
}

impl From<Year> for YearRange {
    fn from(y: Year) -> Self {
        Self { begin: y, end: y }
    }
}

impl From<YearSpec> for YearRange {
    fn from(ys: YearSpec) -> Self {
        match ys {
            YearSpec::SingleYear(y) => y.into(),
            YearSpec::ClosedRange(range) => range,
        }
    }
}

impl Display for YearRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.begin == self.end {
            write!(f, "{}", self.begin.0)
        } else {
            write!(f, "{}-{}", self.begin.0, self.end.0)
        }
    }
}
fn coalesce_years(years: impl IntoIterator<Item = YearRange>) -> impl Iterator<Item = YearRange> {
    years.into_iter().coalesce(|a, b| {
        if a.can_merge(&b) {
            Ok(a.merge_with(b))
        } else {
            Err((a, b))
        }
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum YearSpec {
    /// Just one year (2022)
    SingleYear(Year),
    /// Two years forming a range (2018-2022)
    ClosedRange(YearRange),
    // /// An open-ended year range (2018-)
    // OpenRange(u16),
}

impl Display for YearSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            YearSpec::SingleYear(y) => y.fmt(f),
            YearSpec::ClosedRange(r) => r.fmt(f),
        }
    }
}

impl PartialOrd for YearSpec {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (YearSpec::SingleYear(y), YearSpec::SingleYear(other_y)) => y.partial_cmp(other_y),
            (YearSpec::SingleYear(y), YearSpec::ClosedRange(range)) => {
                range.partial_order_single_year(y).map(|ord| ord.reverse())
            }
            (YearSpec::ClosedRange(range), YearSpec::SingleYear(y)) => {
                range.partial_order_single_year(y)
            }
            (YearSpec::ClosedRange(range), YearSpec::ClosedRange(other_range)) => {
                range.partial_cmp(other_range)
            }
        }
    }
}

impl YearSpec {
    /// Helper to more concisely construct a single year
    pub(crate) fn single(y: u16) -> Self {
        Self::SingleYear(Year(y))
    }

    /// Helper to more concisely construct a closed range of years
    pub(crate) fn range(begin: Year, end: Year) -> Self {
        Self::ClosedRange(YearRange { begin, end })
    }
}
