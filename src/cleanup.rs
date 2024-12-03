// Copyright 2021-2024, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

/// Helpful additions to strings.
pub trait StrExt {
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str>;
}

impl StrExt for str {
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str> {
        re.replace(self, "")
    }
}

pub fn cleanup_copyright_text(text: &Option<String>) -> Vec<Cow<str>> {
    lazy_static! {
        // we don't want the license in the copyright text
        // nor bogus lines
        static ref RE: Regex = Regex::new("(SPDX-License-Identifier:.*|(\\n|,|')+|.*;;;;;;;;;;;;;)$").unwrap();
    }
    text.iter()
        .flat_map(|s| {
            s.split('\n')
                .map(|line| {
                    line.trim()
                        .trim_start_matches("SPDX-FileCopyrightText:")
                        .trim()
                        .trim_start_matches("Copyright")
                        .trim()
                        .trim_start_matches(":")
                        .trim()
                        .trim_start_matches("Copyright")
                        .trim()
                        .trim_start_matches("(c)")
                        .trim_start_matches("(C)")
                        .trim()
                        .trim_end_matches("'")
                        .trim_end_matches("\"")
                        .trim_end_matches("\\n")
                        .trim()
                        .strip_match_if_present(&RE)
                })
                .filter(|str| !str.is_empty())
                .sorted()
        })
        .dedup()
        .collect()
}

pub fn licenses_debian_to_spdx(text: &str) -> String {
    text.replace("Expat", "MIT")
        .replace("BSD-3-clause", "BSD-3-Clause")
}

pub fn licenses_spdx_to_debian(text: &str) -> String {
    text.replace("MIT", "Expat")
        .replace("BSD-3-Clause", "BSD-3-clause")
}
