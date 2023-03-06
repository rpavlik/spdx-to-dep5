// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Operations on bare integer years.
///
/// Important note: Humans use "century" to refer to a 1-indexed number of hundred-year periods since
/// the beginning of the era (year 0). That means technically they're one larger than the year
/// integer-divided by 100. This is a mess and super annoying.
/// TODO: Right now we ignore this and pretend it's not true.

use super::{CENTURY, NINETEEN, TWENTY};

// Ugh. Centuries

pub(crate) fn guess_century(two_digit_year: u16) -> u8 {
    if two_digit_year < 60 {
        TWENTY
    } else {
        NINETEEN
    }
}

pub(crate) fn compose_year(century: u16, two_digit: u16) -> u16 {
    century * CENTURY + two_digit
}

pub(crate) fn guess_four_digit_from_two_digit(two_digit: u16) -> u16 {
    compose_year(u16::from(guess_century(two_digit)), two_digit)
}

pub(crate) fn get_century(year: u16) -> u16 {
    year / CENTURY
}

pub(crate) fn get_two_digit_year(year: u16) -> u16 {
    year % CENTURY
}

#[cfg(test)]
mod tests {

    use crate::raw_year::util::get_two_digit_year;

    use super::get_century;

    #[test]
    fn test_get_century() {
        assert_eq!(get_century(2005), 20);
        assert_eq!(get_century(2105), 21);
        assert_eq!(get_century(1995), 19);
        assert_eq!(get_century(2095), 20);
    }

    #[test]
    fn test_get_two_digit() {
        assert_eq!(get_two_digit_year(2005), 05);
        assert_eq!(get_two_digit_year(2105), 05);
        assert_eq!(get_two_digit_year(1995), 95);
        assert_eq!(get_two_digit_year(2095), 95);
    }
}
