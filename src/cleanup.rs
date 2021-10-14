// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::borrow::Cow;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

/// Helpful additions to strings.
pub trait StrExt {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str;
    fn strip_match_if_present(&self, re: &Regex) -> Cow<str>;
}
impl StrExt for str {
    fn strip_prefix_if_present(&self, prefix: &str) -> &str {
        if let Some(prefix_removed) = self.strip_prefix(prefix) {
            prefix_removed.trim()
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
        static ref RE: Regex = Regex::new("SPDX-License-Identifier:.*$").unwrap();
    }
    text.split('\n')
        .map(|line| {
            line.trim()
                .strip_prefix_if_present("SPDX-FileCopyrightText:")
                .strip_prefix_if_present("Copyright")
                .strip_prefix_if_present("(c)")
                .strip_prefix_if_present("(C)")
                .strip_match_if_present(&RE)
        })
        .sorted()
        .dedup()
        .collect()
}
