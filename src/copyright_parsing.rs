// Copyright 2021-2022, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{multispace0, not_line_ending, space0, space1},
    combinator::{eof, map, recognize, rest, verify},
    multi::separated_list1,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    copyright::{Copyright, DecomposedCopyright},
    raw_year::{self, IsProper, RawYear, RawYearRange},
    years::{Year, YearRange, YearSpec},
};

fn year_spec(input: &str) -> IResult<&str, YearSpec> {
    // preceded and space0 are to remove leading spaces
    preceded(
        space0,
        map(raw_year::parse::year_spec, |(b, e)| {
            if b == e {
                // single year
                YearSpec::SingleYear(Year(b.to_four_digit().into_inner()))
            } else {
                let (b, e) = (b, e).to_four_digit_range();
                assert!((b, e).is_proper());

                YearSpec::ClosedRange(YearRange::new(Year(b.into_inner()), Year(e.into_inner())))
            }
        }),
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
                !s.is_empty()
            }),
            // )),
            // Grab the rest of the line as the holder
            not_line_ending,
        ),
        // Transform the tuple into a DecomposedCopyright
        |(year_spec, holder)| DecomposedCopyright::new(&year_spec, holder),
    )(input)
}

/// For now, just distinguish a single decomposable line from anything else.
/// This will consume all remaining input
pub(crate) fn copyright_lines(input: &str) -> IResult<&str, Copyright> {
    alt((
        map(
            terminated(copyright_line, tuple((multispace0, eof))),
            Copyright::Decomposable,
        ),
        map(preceded(multispace0, rest), |s: &str| {
            Copyright::Complex(s.trim().to_string())
        }),
    ))(input)
}

#[cfg(test)]
mod tests {
    use super::{year_spec, year_spec_vec};
    use crate::years::{Year, YearSpec};
    use nom::{
        combinator::{all_consuming, eof},
        sequence::terminated,
        Finish, IResult,
    };

    #[test]
    fn parse_year_spec() {
        assert_eq!(
            all_consuming(year_spec)("2022").finish().unwrap().1,
            YearSpec::SingleYear(Year(2022))
        );
        assert!(all_consuming(year_spec)("2022-").finish().is_err());

        assert_eq!(
            all_consuming(year_spec)("1995-20").finish().unwrap().1,
            YearSpec::range(Year(1995), Year(2020))
        );

        assert!(all_consuming(year_spec)("1995-1821").is_err());

        assert_eq!(
            all_consuming(year_spec)("1995-2022").finish().unwrap().1,
            YearSpec::range(Year(1995), Year(2022))
        );

        assert_eq!(
            all_consuming(year_spec)("1995 - 2022").finish().unwrap().1,
            YearSpec::range(Year(1995), Year(2022))
        );
        assert_eq!(
            all_consuming(year_spec)("1995").finish().unwrap().1,
            YearSpec::single(1995)
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
        assert!(year_spec_vec_complete("1995-1821").is_err());

        assert_eq!(
            year_spec_vec_complete("1995-20").finish().unwrap().1,
            vec![YearSpec::range(Year(1995), Year(2020))]
        );

        assert_eq!(
            year_spec_vec_complete("1995-2022").finish().unwrap(),
            ("", vec![YearSpec::range(Year(1995), Year(2022))])
        );

        assert_eq!(
            year_spec_vec_complete("1995 - 2022").finish().unwrap(),
            ("", vec![YearSpec::range(Year(1995), Year(2022))])
        );
        assert_eq!(
            year_spec_vec_complete("1995").finish().unwrap(),
            ("", vec![YearSpec::single(1995)])
        );

        assert_eq!(
            year_spec_vec_complete("1995 1996").finish().unwrap(),
            ("", vec![YearSpec::single(1995), YearSpec::single(1996)])
        );
        assert_eq!(
            year_spec_vec_complete("1995, 1996").finish().unwrap(),
            ("", vec![YearSpec::single(1995), YearSpec::single(1996)])
        );

        assert_eq!(
            year_spec_vec_complete("1995, 1996, 1997-2001")
                .finish()
                .unwrap(),
            (
                "",
                vec![
                    YearSpec::single(1995),
                    YearSpec::single(1996),
                    YearSpec::range(Year(1997), Year(2001))
                ]
            )
        );
    }
}
