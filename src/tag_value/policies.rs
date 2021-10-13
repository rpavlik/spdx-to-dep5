// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Policies that may be used with [KVParser] to control its handling of values.

use super::key_value_parser::{ProcessedContinuationValue, ProcessedValue, TagValueParsePolicy};

pub const TEXT_OPEN_TAG: &str = "<text>";
pub const TEXT_CLOSE_TAG: &str = "</text>";

#[derive(Debug, Default, Clone, Copy)]
/// The simplest parse policy, that does no trimming or transformation, and no multi-line values.
pub struct TrivialParsePolicy {}
impl TagValueParsePolicy for TrivialParsePolicy {
    fn process_value<'a>(&self, _key: &str, value: &'a str) -> ProcessedValue<'a> {
        ProcessedValue::CompleteValue(value)
    }

    fn process_continuation<'a>(
        &self,
        _key: &str,
        _continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a> {
        unreachable!()
    }
}

#[derive(Debug, Default, Clone, Copy)]
/// The parse policy used for SPDX Tag-Value files, where a value that starts with `<text>` continues
/// possibly across multiple lines until `</text>`, both of which are trimmed.
pub struct SPDXParsePolicy {}
impl TagValueParsePolicy for SPDXParsePolicy {
    fn process_value<'a>(&self, _key: &str, value: &'a str) -> ProcessedValue<'a> {
        let trimmed_val = value.trim();
        if let Some(value) = trimmed_val.strip_prefix(TEXT_OPEN_TAG) {
            if let Some(value) = value.strip_suffix(TEXT_CLOSE_TAG) {
                // found both open and close
                ProcessedValue::CompleteValue(value)
            } else {
                // only found open
                ProcessedValue::StartOfMultiline(Some(value))
            }
        } else {
            // just plain text
            ProcessedValue::CompleteValue(value)
        }
    }

    fn process_continuation<'a>(
        &self,
        _key: &str,
        continuation_line: &'a str,
    ) -> ProcessedContinuationValue<'a> {
        let line = continuation_line.trim_end();
        if let Some(stripped) = line.strip_suffix(TEXT_CLOSE_TAG) {
            ProcessedContinuationValue::FinishMultiline(Some(stripped))
        } else {
            ProcessedContinuationValue::ContinueMultiline(Some(line))
        }
    }
}
