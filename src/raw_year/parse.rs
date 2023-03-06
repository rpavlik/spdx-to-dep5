// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use clap::Parser;
use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{digit1, multispace0, not_line_ending, one_of, space0, space1},
    combinator::{eof, map, map_res, not, peek, recognize, rest, verify},
    multi::{count, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    IResult,
};

use super::types::{FourDigitYear, RawYear, TwoDigitYear, YearExpr};

fn digit(input: &str) -> IResult<&str, char> {
    one_of("0123456789")(input)
}

fn century(input: &str) -> IResult<&str, &str> {
    alt((tag("19"), tag("20")))(input)
}

fn four_digit_year(input: &str) -> IResult<&str, FourDigitYear> {
    map_res(recognize(pair(century, count(digit, 2))), |out: &str| {
        u16::from_str_radix(&out, 10).map(FourDigitYear::new)
    })(input)
}

fn two_digit_year(input: &str) -> IResult<&str, TwoDigitYear> {
    map_res(recognize(count(digit, 2)), |out: &str| {
        u16::from_str_radix(&out, 10).map(TwoDigitYear::new)
    })(input)
}

fn year(input: &str) -> IResult<&str, YearExpr> {
    alt((
        map(four_digit_year, |y| y.to_year_expr()),
        map(two_digit_year, |y| y.to_year_expr()),
    ))(input)
}

fn range_delim(input: &str) -> IResult<&str, &str> {
    recognize(tuple((space0, tag("-"), space0)))(input)
}

fn convert_range<T: RawYear, U: RawYear>(range: (T, U)) -> (YearExpr, YearExpr) {
    (range.0.to_year_expr(), range.1.to_year_expr())
}

fn year_range(input: &str) -> IResult<&str, (YearExpr, YearExpr)> {
    let range44 = separated_pair(four_digit_year, range_delim, four_digit_year);
    let range42 = separated_pair(four_digit_year, range_delim, two_digit_year);
    let range24 = separated_pair(two_digit_year, range_delim, four_digit_year);
    let range22 = separated_pair(two_digit_year, range_delim, two_digit_year);
    alt((
        map(range44, convert_range),
        map(range42, convert_range),
        map(range24, convert_range),
        map(range22, convert_range),
    ))(input)
}

fn year_spec(input: &str) -> IResult<&str, (YearExpr, YearExpr)> {
    // preceded and space0 are to remove leading spaces
    preceded(
        space0,
        alt((
            // could be a year range: always try this first
            year_range,
            // Failing that, could be a single year (represented by a range with same begin and end)
            map(year, |y| (y, y)),
        )),
    )(input)
}

#[cfg(test)]
mod tests {
    use nom::{combinator::all_consuming, Finish};

    use crate::raw_year::types::{FourDigitYear, RawYear, YearExpr};

    use super::{four_digit_year, two_digit_year, year};

    #[test]
    fn parse_year() {
        // assert_finished_and_eq!(year("1995"))
        assert!(all_consuming(four_digit_year)("20").is_err());
        assert!(all_consuming(year)("202").is_err());
        assert!(all_consuming(four_digit_year)("19").is_err());
        assert!(all_consuming(year)("199").is_err());

        assert_eq!(
            all_consuming(year)("20")
                .finish()
                .unwrap()
                .1
                .to_four_digit(),
            FourDigitYear::new(2020)
        );
        assert_eq!(
            all_consuming(year)("2022")
                .finish()
                .unwrap()
                .1
                .to_four_digit(),
            FourDigitYear::new(2022)
        );
        assert_eq!(
            all_consuming(year)("19")
                .finish()
                .unwrap()
                .1
                .to_four_digit(),
            FourDigitYear::new(2019)
        );
        assert_eq!(
            all_consuming(year)("2022")
                .finish()
                .unwrap()
                .1
                .to_four_digit(),
            FourDigitYear::new(2022)
        );
        assert_eq!(
            all_consuming(year)("1995")
                .finish()
                .unwrap()
                .1
                .to_four_digit(),
            FourDigitYear::new(1995)
        );
        assert!(all_consuming(year)("20222").finish().is_err());
    }
}
