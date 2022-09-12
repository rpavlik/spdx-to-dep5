// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{one_of, space0},
    combinator::{map, map_res, recognize},
    multi::count,
    sequence::{pair, preceded, separated_pair, tuple},
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
        out.parse::<u16>().map(FourDigitYear::new)
    })(input)
}

fn two_digit_year(input: &str) -> IResult<&str, TwoDigitYear> {
    map_res(recognize(count(digit, 2)), |out: &str| {
        out.parse::<u16>().map(TwoDigitYear::new)
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

pub(crate) fn year_range_44(input: &str) -> IResult<&str, (FourDigitYear, FourDigitYear)> {
    separated_pair(four_digit_year, range_delim, four_digit_year)(input)
}

pub(crate) fn year_range_42(input: &str) -> IResult<&str, (FourDigitYear, TwoDigitYear)> {
    separated_pair(four_digit_year, range_delim, two_digit_year)(input)
}

pub(crate) fn year_range_24(input: &str) -> IResult<&str, (TwoDigitYear, FourDigitYear)> {
    separated_pair(two_digit_year, range_delim, four_digit_year)(input)
}

pub(crate) fn year_range_22(input: &str) -> IResult<&str, (TwoDigitYear, TwoDigitYear)> {
    separated_pair(two_digit_year, range_delim, two_digit_year)(input)
}

pub(crate) fn year_range(input: &str) -> IResult<&str, (YearExpr, YearExpr)> {
    alt((
        map(year_range_44, convert_range),
        map(year_range_42, convert_range),
        map(year_range_24, convert_range),
        map(year_range_22, convert_range),
    ))(input)
}

pub(crate) fn year_spec(input: &str) -> IResult<&str, (YearExpr, YearExpr)> {
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

    use super::{four_digit_year, two_digit_year, year, year_range, year_range_44};

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

    #[test]
    fn parse_four_digit_year() {
        // assert_finished_and_eq!(year("1995"))
        assert!(four_digit_year("202").is_err());
        assert!(four_digit_year("20").is_err());
        assert!(four_digit_year("199").is_err());
        assert!(four_digit_year("19").is_err());

        assert_eq!(
            four_digit_year("2022").finish().unwrap(),
            ("", FourDigitYear::new(2022))
        );
        assert_eq!(
            four_digit_year("2022").finish().unwrap(),
            ("", FourDigitYear::new(2022))
        );
        assert_eq!(
            four_digit_year("1995").finish().unwrap(),
            ("", FourDigitYear::new(1995))
        );
        assert!(all_consuming(four_digit_year)("20222").finish().is_err());
    }

    #[test]
    fn parse_two_digityear() {
        assert!(all_consuming(two_digit_year)("202").finish().is_err());
        assert!(all_consuming(two_digit_year)("2020").finish().is_err());
        assert!(all_consuming(two_digit_year)("199").finish().is_err());
        assert!(all_consuming(two_digit_year)("1995").finish().is_err());

        assert_eq!(
            two_digit_year("20").finish().unwrap().1.to_four_digit(),
            FourDigitYear::new(2020)
        );
        assert_eq!(
            two_digit_year("19").finish().unwrap().1.to_four_digit(),
            FourDigitYear::new(2019)
        );
        assert_eq!(
            two_digit_year("85").finish().unwrap().1.to_four_digit(),
            FourDigitYear::new(1985)
        );
    }

    #[test]
    fn parse_year2() {
        // assert_finished_and_eq!(year("1995"))
        assert!(all_consuming(year)("202").is_err());
        assert!(all_consuming(four_digit_year)("20").is_err());
        assert!(all_consuming(year)("199").is_err());
        assert!(all_consuming(four_digit_year)("19").is_err());

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

    #[test]
    fn parse_year_range() {
        // assert_finished_and_eq!(year("1995"))
        assert!(all_consuming(year_range)("2022").finish().is_err());
        assert!(all_consuming(year_range)("2022-").finish().is_err());
        assert!(all_consuming(year_range)("1995-1821").finish().is_err());

        assert_eq!(
            all_consuming(year_range)("1995-20").finish().unwrap().1,
            (YearExpr::new_four_digit(1995), YearExpr::new_two_digit(20))
        );

        assert_eq!(
            all_consuming(year_range_44)("1995-2022")
                .finish()
                .unwrap()
                .1,
            (FourDigitYear::new(1995), FourDigitYear::new(2022))
        );

        assert_eq!(
            all_consuming(year_range)("1995-2022").finish().unwrap().1,
            (
                YearExpr::new_four_digit(1995),
                YearExpr::new_four_digit(2022)
            )
        );

        assert_eq!(
            all_consuming(year_range_44)("1995 - 2022")
                .finish()
                .unwrap()
                .1,
            (FourDigitYear::new(1995), FourDigitYear::new(2022))
        );

        assert_eq!(
            all_consuming(year_range)("1995 - 2022").finish().unwrap().1,
            (
                YearExpr::new_four_digit(1995),
                YearExpr::new_four_digit(2022)
            )
        );
    }
}
