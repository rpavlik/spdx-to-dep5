// Copyright 2021-2024, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use spdx_rs::models::FileInformation;

fn is_copyright_text_empty(fi: &FileInformation) -> bool {
    match &fi.copyright_text {
        None => true,
        Some(v) => v == "NONE",
    }
}

fn omit_no_copyright(file_info: Vec<FileInformation>) -> Vec<FileInformation> {
    file_info
        .into_iter()
        .filter(|f| !is_copyright_text_empty(f))
        .collect()
}

fn normalize_no_copyright(file_info: Vec<FileInformation>) -> Vec<FileInformation> {
    file_info
        .into_iter()
        .map(|f| {
            if is_copyright_text_empty(&f) {
                let mut f = f;
                f.copyright_text = None;
                f
            } else {
                f
            }
        })
        .collect()
}

pub fn omit_or_normalize_none(
    file_info: Vec<FileInformation>,
    omit_missing_copyright: bool,
) -> Vec<FileInformation> {
    if omit_missing_copyright {
        omit_no_copyright(file_info)
    } else {
        normalize_no_copyright(file_info)
    }
}
