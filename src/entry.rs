// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::convert::TryFrom;

use crate::{key_value_parser::ParsedLine, record::{Record, RecordError}};

struct SpdxId(String);

enum Checksum {
    SHA1(String),
}
struct Entry {
    filename: String,
    id: SpdxId,
    fileChecksum: Checksum,
    licenseConcluded: Option<String>,
    licenseInfoInFile: String,
    fileCopyrightText: String,
}

const KEY_FILENAME: &str = &"FileName";
const KEY_SPDXID: &str = &"SPDXID";
const KEY_FILECHECKSUM: &str = &"FileChecksum";
const KEY_LICENSECONCLUDED: &str = &"LicenseConcluded";
const KEY_LICENSEINFOINFILE: &str = &"LicenseInfoInFile";
const KEY_FILECOPYRIGHTTEXT: &str = &"FileCopyrightText";

impl TryFrom<Record> for Entry {
    type Error = RecordError;

    fn try_from(record: Record) -> Result<Self, Self::Error> {
        let filename = record.value_for_required_key(KEY_FILENAME)?;
        let id = record.value_for_required_key(KEY_SPDXID)?;
    }
}
fn parse_checksum(value: &String) -> Option<Checksum> {
    match ParsedLine::from(value) {
        ParsedLine::RecordDelimeter => None,
        ParsedLine::ValueOnly(_) => None,
        ParsedLine::KVPair(pair) => {
            if pair.key == "SHA1" {
                Some(Checksum::SHA1(pair.value.to_string()))
            } else {
                None
            }
        }
    }
}
