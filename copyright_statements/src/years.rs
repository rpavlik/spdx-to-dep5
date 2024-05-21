// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::BinaryHeap, fmt::Display};

use itertools::Itertools;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Year(pub u16);

impl Display for Year {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Whether something "contains" a year or year range.
pub trait YearContainment {
    /// Is this single year included in this?
    fn contains_year(&self, other: &Year) -> bool;
    /// Is this range included in this?
    fn contains_range(&self, other: &YearRange) -> bool;
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

    pub fn begin(&self) -> Year {
        self.begin
    }

    pub fn end(&self) -> Year {
        self.end
    }

    pub fn is_single_year(&self) -> bool {
        self.begin == self.end
    }

    fn can_add(&self, new_year: &Year) -> bool {
        // within the range
        self.contains_year(new_year)
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

pub fn coalesce_years(
    years: impl IntoIterator<Item = YearRange>,
) -> impl Iterator<Item = YearRange> {
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

    pub fn contains(&self, other: &YearSpec) -> bool {
        match other {
            YearSpec::SingleYear(y) => self.contains_year(y),
            YearSpec::ClosedRange(r) => self.contains_range(r),
        }
    }
}

impl YearContainment for YearRange {
    fn contains_year(&self, other: &Year) -> bool {
        other <= &self.end && other >= &self.begin
    }

    fn contains_range(&self, other: &YearRange) -> bool {
        self.contains_year(&other.begin) && self.contains_year(&other.end)
    }
}

impl YearContainment for Year {
    fn contains_year(&self, other: &Year) -> bool {
        self == other
    }

    fn contains_range(&self, other: &YearRange) -> bool {
        *self == other.begin && *self == other.end
    }
}

impl YearContainment for YearSpec {
    fn contains_year(&self, other: &Year) -> bool {
        match self {
            YearSpec::SingleYear(y) => y.contains_year(other),
            YearSpec::ClosedRange(r) => r.contains_year(other),
        }
    }

    fn contains_range(&self, other: &YearRange) -> bool {
        match self {
            YearSpec::SingleYear(y) => y.contains_range(other),
            YearSpec::ClosedRange(r) => r.contains_range(other),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TotalOrderedYearRange(YearRange);

impl TotalOrderedYearRange {
    fn make_key(&self) -> (i32, i32) {
        // convert them to signed, and negate the end so that larger ranges (with higher "end" values) sort first.
        (i32::from(self.0.begin().0), -i32::from(self.0.end().0))
    }
}

impl Ord for TotalOrderedYearRange {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.make_key().cmp(&other.make_key())
    }
}

impl PartialOrd for TotalOrderedYearRange {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<YearRange> for TotalOrderedYearRange {
    fn from(value: YearRange) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Default)]
pub struct YearRangeCollection {
    years_heap: BinaryHeap<TotalOrderedYearRange>,
}

impl YearRangeCollection {
    pub fn new() -> Self {
        YearRangeCollection::default()
    }
    pub fn accumulate(&mut self, year_spec: YearSpec) {
        self.years_heap
            .push(TotalOrderedYearRange::from(YearRange::from(year_spec)));
    }
    pub fn into_coalesced_vec(self) -> Vec<YearRange> {
        coalesce_years(
            self.years_heap
                .into_sorted_vec()
                .into_iter()
                .map(|tosr| tosr.0),
        )
        .collect()
    }
}

impl Extend<YearSpec> for YearRangeCollection {
    fn extend<T: IntoIterator<Item = YearSpec>>(&mut self, iter: T) {
        self.years_heap.extend(
            iter.into_iter()
                .map(YearRange::from)
                .map(TotalOrderedYearRange::from),
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn year_and_year_range_contains() {
        let year_2024 = Year(2024);
        let year_2025 = Year(2025);
        let year_2023 = Year(2023);
        let range_2024 = YearRange::new(year_2024, year_2024);
        let range_2024_2025 = YearRange::new(year_2024, year_2025);
        let range_2023_2024 = YearRange::new(year_2023, year_2024);

        // 2024 only
        assert!(year_2024.contains_year(&year_2024));
        assert!(!year_2024.contains_year(&year_2025));
        assert!(!year_2024.contains_year(&year_2023));

        assert!(year_2024.contains_range(&range_2024));
        assert!(!year_2024.contains_range(&range_2024_2025));
        assert!(!year_2024.contains_range(&range_2023_2024));

        assert!(range_2024.contains_year(&year_2024));
        assert!(!range_2024.contains_year(&year_2025));
        assert!(!range_2024.contains_year(&year_2023));

        assert!(range_2024.contains_range(&range_2024));
        assert!(!range_2024.contains_range(&range_2024_2025));
        assert!(!range_2024.contains_range(&range_2023_2024));

        // 2024-2025
        assert!(range_2024_2025.contains_year(&year_2024));
        assert!(range_2024_2025.contains_year(&year_2025));
        assert!(!range_2024_2025.contains_year(&year_2023));

        assert!(range_2024_2025.contains_range(&range_2024));
        assert!(range_2024_2025.contains_range(&range_2024_2025));
        assert!(!range_2024_2025.contains_range(&range_2023_2024));

        // 2023-2024
        assert!(range_2023_2024.contains_year(&year_2024));
        assert!(!range_2023_2024.contains_year(&year_2025));
        assert!(range_2023_2024.contains_year(&year_2023));

        assert!(range_2023_2024.contains_range(&range_2024));
        assert!(!range_2023_2024.contains_range(&range_2024_2025));
        assert!(range_2023_2024.contains_range(&range_2023_2024));
    }

    #[test]
    fn year_spec_contains() {
        let year_2024 = Year(2024);
        let year_2025 = Year(2025);
        let year_2023 = Year(2023);
        let range_2024 = YearRange::new(year_2024, year_2024);
        let range_2024_2025 = YearRange::new(year_2024, year_2025);
        let range_2023_2024 = YearRange::new(year_2023, year_2024);

        let year_spec_2024 = YearSpec::single(2024);
        let range_spec_2024 = YearSpec::range(Year(2024), Year(2024));
        let range_spec_2024_2025 = YearSpec::range(Year(2024), Year(2025));
        let range_spec_2023_2024 = YearSpec::range(Year(2023), Year(2024));
        // 2024 only
        assert!(year_spec_2024.contains_year(&year_2024));
        assert!(!year_spec_2024.contains_year(&year_2025));
        assert!(!year_spec_2024.contains_year(&year_2023));

        assert!(year_spec_2024.contains_range(&range_2024));
        assert!(!year_spec_2024.contains_range(&range_2024_2025));
        assert!(!year_spec_2024.contains_range(&range_2023_2024));

        assert!(range_spec_2024.contains_year(&year_2024));
        assert!(!range_spec_2024.contains_year(&year_2025));
        assert!(!range_spec_2024.contains_year(&year_2023));

        assert!(range_spec_2024.contains_range(&range_2024));
        assert!(!range_spec_2024.contains_range(&range_2024_2025));
        assert!(!range_spec_2024.contains_range(&range_2023_2024));

        // 2024-2025
        assert!(range_spec_2024_2025.contains_year(&year_2024));
        assert!(range_spec_2024_2025.contains_year(&year_2025));
        assert!(!range_spec_2024_2025.contains_year(&year_2023));

        assert!(range_spec_2024_2025.contains_range(&range_2024));
        assert!(range_spec_2024_2025.contains_range(&range_2024_2025));
        assert!(!range_spec_2024_2025.contains_range(&range_2023_2024));

        // 2023-2024
        assert!(range_spec_2023_2024.contains_year(&year_2024));
        assert!(!range_spec_2023_2024.contains_year(&year_2025));
        assert!(range_spec_2023_2024.contains_year(&year_2023));

        assert!(range_spec_2023_2024.contains_range(&range_2024));
        assert!(!range_spec_2023_2024.contains_range(&range_2024_2025));
        assert!(range_spec_2023_2024.contains_range(&range_2023_2024));
    }
}
