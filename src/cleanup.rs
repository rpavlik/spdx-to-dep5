// Copyright 2021-2024, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

/// Helpful additions to strings.
pub trait StrExt {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str;
    fn strip_suffix_if_present(&self, suffix: &str) -> &str;
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str>;
}

impl StrExt for str {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str {
        if let Some(after_removed) = self.strip_prefix(prefix) {
            after_removed.trim()
        } else {
            self
        }
    }
    fn strip_suffix_if_present(&self, suffix: &str) -> &str {
        if let Some(after_removed) = self.strip_suffix(suffix) {
            after_removed.trim()
        } else {
            self
        }
    }
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
                        .strip_prefix_if_present("SPDX-FileCopyrightText:")
                        .strip_prefix_if_present("Copyright")
                        .strip_prefix_if_present(":")
                        .strip_prefix_if_present("Copyright")
                        .strip_prefix_if_present("(c)")
                        .strip_prefix_if_present("(C)")
                        .strip_suffix_if_present("'")
                        .strip_suffix_if_present("\"")
                        .strip_suffix_if_present("\\n")
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
