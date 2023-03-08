// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
/// Operations on bare integer years.
///
/// Important note: Humans use "century" to refer to a 1-indexed number of hundred-year periods since
/// the beginning of the era (year 0). That means technically they're one larger than the year
/// integer-divided by 100. This is a mess and super annoying.
use super::CENTURY_DURATION;

// Ugh. Centuries

pub(crate) fn guess_century(two_digit_year: u16) -> u8 {
    if two_digit_year < 60 {
        21
    } else {
        20
    }
}

pub(crate) fn compose_year(century: u16, two_digit: u16) -> u16 {
    (century - 1) * CENTURY_DURATION + two_digit
}

pub(crate) fn guess_four_digit_from_two_digit(two_digit: u16) -> u16 {
    compose_year(u16::from(guess_century(two_digit)), two_digit)
}

pub(crate) fn get_century(year: u16) -> u16 {
    year / CENTURY_DURATION + 1
}

pub(crate) fn get_two_digit_year(year: u16) -> u16 {
    year % CENTURY_DURATION
}

#[cfg(test)]
mod tests {

    use crate::raw_year::util::get_two_digit_year;

    use super::get_century;

    #[test]
    fn test_get_century() {
        assert_eq!(get_century(2005), 21);
        assert_eq!(get_century(2105), 22);
        assert_eq!(get_century(1995), 20);
        assert_eq!(get_century(2095), 21);
    }

    #[test]
    fn test_get_two_digit() {
        assert_eq!(get_two_digit_year(2005), 05);
        assert_eq!(get_two_digit_year(2105), 05);
        assert_eq!(get_two_digit_year(1995), 95);
        assert_eq!(get_two_digit_year(2095), 95);
    }
}
