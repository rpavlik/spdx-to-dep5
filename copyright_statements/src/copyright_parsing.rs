// Copyright 2021-2023, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{multispace0, not_line_ending, space0, space1},
    combinator::{eof, map, map_opt, opt, recognize, rest, value, verify},
    multi::{many1, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::{
    copyright::{Copyright, DecomposedCopyright},
    raw_year::{
        self,
        traits::{ConfigurableRawYearRange, YearRangeNormalizationOptions},
        RawYear,
    },
    years::{Year, YearRange, YearSpec},
};

fn year_spec(
    options: impl YearRangeNormalizationOptions + Copy,
) -> impl FnMut(&str) -> IResult<&str, YearSpec> {
    // preceded and space0 are to remove leading spaces
    move |input: &str| {
        preceded(
            space0,
            map_opt(raw_year::parse::year_spec, |(b, e)| {
                if b == e {
                    // single year
                    Some(YearSpec::SingleYear(Year(b.to_four_digit().into_inner())))
                } else {
                    (b, e).try_to_four_digit_range(options).map(|(b, e)| {
                        YearSpec::ClosedRange(YearRange::new(
                            Year(b.into_inner()),
                            Year(e.into_inner()),
                        ))
                    })
                }
            }),
        )(input)
    }
}

fn year_spec_vec(
    options: impl YearRangeNormalizationOptions + Copy,
) -> impl FnMut(&str) -> IResult<&str, Vec<YearSpec>> {
    move |input: &str| {
        separated_list1(
            alt((delimited(space0, tag(","), space0), space1)),
            year_spec(options),
        )(input)
    }
}

fn copyright_prefix() -> impl FnMut(&str) -> IResult<&str, ()> {
    move |input: &str| {
        value(
            (),
            opt(tuple((
                multispace0,
                alt((
                    tag_no_case("copyright"),
                    tag_no_case("copyright (C)"),
                    tag_no_case("copr"),
                )),
                multispace0,
            ))),
        )(input)
    }
}

fn copyright_line(
    options: impl YearRangeNormalizationOptions + Copy,
) -> impl FnMut(&str) -> IResult<&str, DecomposedCopyright> {
    move |input: &str| {
        map(
            preceded(
                // Might say "copyright" first
                copyright_prefix(),
                separated_pair(
                    // Grab our years
                    year_spec_vec(options),
                    // could be separated by a comma with some optional spaces
                    verify(recognize(tuple((space0, tag(","), space0))), |s: &str| {
                        !s.is_empty()
                    }),
                    // Grab the rest of the line as the holder
                    not_line_ending,
                ),
            ),
            // Transform the tuple into a DecomposedCopyright
            |(year_spec, holder)| DecomposedCopyright::new(&year_spec, holder),
        )(input)
    }
}

/// For now, just distinguish a single decomposable line from anything else.
/// This will consume all remaining input
pub(crate) fn copyright_lines(
    options: impl YearRangeNormalizationOptions + Copy,
) -> impl FnMut(&str) -> IResult<&str, Copyright> {
    move |input: &str| {
        alt((
            map(
                many1(terminated(copyright_line(options), multispace0)),
                |mut parsed| {
                    if parsed.len() == 1 {
                        Copyright::Decomposable(parsed.pop().expect("always there"))
                    } else {
                        Copyright::MultilineDecomposable(parsed)
                    }
                },
            ),
            map(
                terminated(copyright_line(options), tuple((multispace0, eof))),
                Copyright::Decomposable,
            ),
            map(preceded(multispace0, rest), |s: &str| {
                Copyright::Complex(s.trim().to_string())
            }),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        raw_year::{options::YearRangeNormalization, traits::SetYearRangeNormalizationOptions},
        years::{Year, YearSpec},
    };
    use nom::{combinator::all_consuming, Finish};

    #[test]
    fn parse_year_spec() {
        let mut all_year_spec_configured =
            all_consuming(year_spec(YearRangeNormalization::default()));
        assert_eq!(
            all_year_spec_configured("2022").finish().unwrap().1,
            YearSpec::SingleYear(Year(2022))
        );
        assert!(all_year_spec_configured("2022-").finish().is_err());

        assert!(all_year_spec_configured("1995-20").finish().is_err());
        assert_eq!(
            all_consuming(year_spec(
                YearRangeNormalization::default().allow_mixed_size_implied_century_rollover(true)
            ))("1995-20")
            .finish()
            .unwrap()
            .1,
            YearSpec::range(Year(1995), Year(2020))
        );

        assert!(all_year_spec_configured("1995-1821").is_err());

        assert_eq!(
            all_year_spec_configured("1995-2022").finish().unwrap().1,
            YearSpec::range(Year(1995), Year(2022))
        );

        assert_eq!(
            all_year_spec_configured("1995 - 2022").finish().unwrap().1,
            YearSpec::range(Year(1995), Year(2022))
        );
        assert_eq!(
            all_year_spec_configured("1995").finish().unwrap().1,
            YearSpec::single(1995)
        );
    }

    #[test]
    fn parse_year_spec_vec() {
        let opt = YearRangeNormalization::default;

        assert_eq!(
            all_consuming(year_spec_vec(opt()))("2022").unwrap().1,
            vec![YearSpec::SingleYear(Year(2022))]
        );
        assert!(all_consuming(year_spec_vec(opt()))("2022-").is_err());
        assert!(all_consuming(year_spec_vec(opt()))("1995-1821").is_err());

        assert!(all_consuming(year_spec_vec(opt()))("1995-20")
            .finish()
            .is_err());
        assert_eq!(
            all_consuming(year_spec_vec(
                opt().allow_mixed_size_implied_century_rollover(true)
            ))("1995-20")
            .finish()
            .unwrap()
            .1,
            vec![YearSpec::range(Year(1995), Year(2020))]
        );

        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995-2022")
                .finish()
                .unwrap()
                .1,
            vec![YearSpec::range(Year(1995), Year(2022))]
        );

        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995 - 2022")
                .finish()
                .unwrap()
                .1,
            vec![YearSpec::range(Year(1995), Year(2022))]
        );
        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995")
                .finish()
                .unwrap()
                .1,
            vec![YearSpec::single(1995)]
        );

        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995 1996")
                .finish()
                .unwrap()
                .1,
            vec![YearSpec::single(1995), YearSpec::single(1996)]
        );
        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995, 1996")
                .finish()
                .unwrap()
                .1,
            vec![YearSpec::single(1995), YearSpec::single(1996)]
        );

        assert_eq!(
            all_consuming(year_spec_vec(opt()))("1995, 1996, 1997-2001")
                .finish()
                .unwrap()
                .1,
            vec![
                YearSpec::single(1995),
                YearSpec::single(1996),
                YearSpec::range(Year(1997), Year(2001))
            ]
        );
    }

    #[test]
    fn test_line() {
        let opt = YearRangeNormalization::default;
        assert_eq!(
            all_consuming(copyright_line(opt()))("Copyright 2024, Rylie Pavlik")
                .finish()
                .unwrap()
                .1,
            DecomposedCopyright::new_from_single_yearspec(&YearSpec::single(2024), "Rylie Pavlik")
        );

        assert_eq!(
            all_consuming(copyright_line(opt()))("2024, Rylie Pavlik")
                .finish()
                .unwrap()
                .1,
            DecomposedCopyright::new_from_single_yearspec(&YearSpec::single(2024), "Rylie Pavlik")
        );
    }
}
