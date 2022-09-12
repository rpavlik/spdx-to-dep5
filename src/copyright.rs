// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nom::{
    branch::alt,
    bytes::complete::{tag, take_till, take_until},
    character::complete::{line_ending, multispace0, not_line_ending, one_of, space0, space1},
    combinator::{consumed, eof, map, map_res, recognize, rest, verify},
    multi::{count, many1, separated_list1},
    sequence::{delimited, pair, preceded, separated_pair, terminated, tuple},
    Finish, IResult,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Year(u16);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum YearSpec {
    /// Just one year (2022)
    SingleYear(Year),
    /// Two years forming a range (2018-2022)
    ClosedRange(Year, Year),
    // /// An open-ended year range (2018-)
    // OpenRange(u16),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct DecomposedCopyright {
    years: Vec<YearSpec>,
    holder: String,
}

fn year(input: &str) -> IResult<&str, Year> {
    map_res(
        recognize(pair(
            alt((tag("19"), tag("20"))),
            count(one_of("0123456789"), 2),
        )),
        |out: &str| u16::from_str_radix(&out, 10).map(Year),
    )(input)
}

fn year_range(input: &str) -> IResult<&str, YearSpec> {
    map(
        separated_pair(year, tuple((space0, tag("-"), space0)), year),
        |(begin_year, end_year)| YearSpec::ClosedRange(begin_year, end_year),
    )(input)
}
fn year_spec(input: &str) -> IResult<&str, YearSpec> {
    // preceded and space0 are to remove leading spaces
    preceded(
        space0,
        alt((
            // could be a year range: always try this first
            year_range,
            // Failing that, could be a single year
            map(year, |y| YearSpec::SingleYear(y)),
        )),
    )(input)
}

fn year_spec_vec(input: &str) -> IResult<&str, Vec<YearSpec>> {
    separated_list1(
        alt((delimited(space0, tag(","), space0), space1)),
        year_spec,
    )(input)
}

fn copyright_line(input: &str) -> IResult<&str, DecomposedCopyright> {
    map(
        separated_pair(
            // Grab our years
            year_spec_vec,
            // alt((
            //     // could be separated just by spaces
            //     space1,
            // could be separated by a comma with some optional spaces
            verify(recognize(tuple((space0, tag(","), space0))), |s: &str| {
                s.len() > 0
            }),
            // )),
            // Grab the rest of the line as the holder
            not_line_ending,
        ),
        // Transform the tuple into a DecomposedCopyright
        |(year_spec, holder)| DecomposedCopyright {
            years: year_spec,
            holder: holder.trim().to_string(),
        },
    )(input)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Copyright {
    Decomposable(DecomposedCopyright),
    Complex(String),
}

/// For now, just distinguish a single decomposable line from anything else.
/// This will consume all remaining input
fn copyright_lines(input: &str) -> IResult<&str, Copyright> {
    alt((
        map(
            terminated(copyright_line, tuple((multispace0, eof))),
            |decomposed| Copyright::Decomposable(decomposed),
        ),
        map(preceded(multispace0, rest), |s: &str| {
            Copyright::Complex(s.trim().to_string())
        }),
    ))(input)
}

impl Copyright {
    fn try_parse(statement: &str) -> Result<Self, nom::error::Error<&str>> {
        copyright_lines(statement)
            .finish()
            .map(|(_leftover, parsed)| parsed)
    }
}

#[cfg(test)]
mod tests {
    use crate::copyright::{year, year_range, year_spec, Year, YearSpec};
    use nom::{combinator::eof, sequence::terminated, Finish, IResult};

    use super::year_spec_vec;

    #[test]
    fn parse_year() {
        // assert_finished_and_eq!(year("1995"))
        assert!(year("20").is_err());
        assert!(year("202").is_err());
        assert!(year("19").is_err());
        assert!(year("199").is_err());

        assert_eq!(year("2022").finish().unwrap(), ("", Year(2022)));
        assert_eq!(year("2022").finish().unwrap(), ("", Year(2022)));
        assert_eq!(year("1995").finish().unwrap(), ("", Year(1995)));
        assert!(terminated(year, eof)("20222").finish().is_err());
    }

    #[test]
    fn parse_year_range() {
        // assert_finished_and_eq!(year("1995"))
        assert!(year_range("2022").is_err());
        assert!(year_range("2022-").is_err());
        assert!(year_range("1995-20").is_err());
        assert!(year_range("1995-1821").is_err());

        assert_eq!(
            year_range("1995-2022").finish().unwrap(),
            ("", YearSpec::ClosedRange(Year(1995), Year(2022)))
        );

        assert_eq!(
            year_range("1995 - 2022").finish().unwrap(),
            ("", YearSpec::ClosedRange(Year(1995), Year(2022)))
        );
    }
    fn year_spec_complete(input: &str) -> IResult<&str, YearSpec> {
        terminated(year_spec, eof)(input)
    }
    #[test]
    fn parse_year_spec() {
        assert_eq!(
            year_spec("2022").unwrap(),
            ("", YearSpec::SingleYear(Year(2022)))
        );
        assert!(year_spec_complete("2022-").is_err());
        assert!(year_spec_complete("1995-20").is_err());
        assert!(year_spec_complete("1995-1821").is_err());

        assert_eq!(
            year_spec_complete("1995-2022").finish().unwrap(),
            ("", YearSpec::ClosedRange(Year(1995), Year(2022)))
        );

        assert_eq!(
            year_spec_complete("1995 - 2022").finish().unwrap(),
            ("", YearSpec::ClosedRange(Year(1995), Year(2022)))
        );
        assert_eq!(
            year_spec_complete("1995").finish().unwrap(),
            ("", YearSpec::SingleYear(Year(1995)))
        );
    }

    fn year_spec_vec_complete(input: &str) -> IResult<&str, Vec<YearSpec>> {
        terminated(year_spec_vec, eof)(input)
    }

    #[test]
    fn parse_year_spec_vec() {
        assert_eq!(
            year_spec_vec("2022").unwrap(),
            ("", vec![YearSpec::SingleYear(Year(2022))])
        );
        assert!(year_spec_vec_complete("2022-").is_err());
        assert!(year_spec_vec_complete("1995-20").is_err());
        assert!(year_spec_vec_complete("1995-1821").is_err());

        assert_eq!(
            year_spec_vec_complete("1995-2022").finish().unwrap(),
            ("", vec![YearSpec::ClosedRange(Year(1995), Year(2022))])
        );

        assert_eq!(
            year_spec_vec_complete("1995 - 2022").finish().unwrap(),
            ("", vec![YearSpec::ClosedRange(Year(1995), Year(2022))])
        );
        assert_eq!(
            year_spec_vec_complete("1995").finish().unwrap(),
            ("", vec![YearSpec::SingleYear(Year(1995))])
        );

        assert_eq!(
            year_spec_vec_complete("1995 1996").finish().unwrap(),
            (
                "",
                vec![
                    YearSpec::SingleYear(Year(1995)),
                    YearSpec::SingleYear(Year(1996))
                ]
            )
        );
        assert_eq!(
            year_spec_vec_complete("1995, 1996").finish().unwrap(),
            (
                "",
                vec![
                    YearSpec::SingleYear(Year(1995)),
                    YearSpec::SingleYear(Year(1996))
                ]
            )
        );

        assert_eq!(
            year_spec_vec_complete("1995, 1996, 1997-2001")
                .finish()
                .unwrap(),
            (
                "",
                vec![
                    YearSpec::SingleYear(Year(1995)),
                    YearSpec::SingleYear(Year(1996)),
                    YearSpec::ClosedRange(Year(1997), Year(2001))
                ]
            )
        );
    }
}
