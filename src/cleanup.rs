// Copyright 2021-2022, Collabora, Ltd.
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

pub fn cleanup_copyright_text(text: &str) -> Vec<Cow<str>> {
    lazy_static! {
        // we don't want the license in the copyright text
        static ref RE: Regex = Regex::new("(SPDX-License-Identifier:.*|(\\n|,|')+)$").unwrap();
    }
    text.split('\n')
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
        .filter_map(|str| if str.is_empty() { None } else { Some(str) })
        .sorted()
        .dedup()
        .collect()
}
