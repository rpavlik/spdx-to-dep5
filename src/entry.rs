// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::{convert::TryFrom, str::FromStr};

use chrono::DateTime;
use regex::{Captures, Regex};
use serde::{de, de::value::BorrowedStrDeserializer, Deserialize};
use spdx_rs::models::{self, Algorithm};

use crate::{
    key_value_parser::ParsedLine,
    record::{Record, RecordError},
};

struct SpdxId(String);

// enum Checksum {
//     SHA1(String),
// }
struct Entry {
    filename: String,
    id: SpdxId,
    fileChecksum: Vec<Checksum>,
    licenseConcluded: Option<String>,
    licenseInfoInFile: String,
    fileCopyrightText: String,
}

impl de::Error for RecordError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        RecordError::Message(msg.to_string())
    }
}

const KEY_FILENAME: &str = &"FileName";
const KEY_SPDXID: &str = &"SPDXID";
const KEY_FILECHECKSUM: &str = &"FileChecksum";
const KEY_LICENSECONCLUDED: &str = &"LicenseConcluded";
const KEY_LICENSEINFOINFILE: &str = &"LicenseInfoInFile";
const KEY_FILECOPYRIGHTTEXT: &str = &"FileCopyrightText";

struct Checksum(models::Checksum);

impl TryFrom<&str> for Checksum {
    type Error = RecordError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let pair = ParsedLine::from(value).pair().ok_or(RecordError::Message(
            "Could not parse checksum field".to_string(),
        ))?;
        let d: BorrowedStrDeserializer<Self::Error> = BorrowedStrDeserializer::new(&pair.key);
        let algorithm: Algorithm = models::Algorithm::deserialize(d)?;
        Ok(Checksum(models::Checksum {
            algorithm,
            value: pair.value,
        }))
    }
}

impl TryFrom<Record> for models::FileInformation {
    type Error = RecordError;

    fn try_from(record: Record) -> Result<Self, Self::Error> {
        let file_name = record.value_for_required_key(KEY_FILENAME)?.clone();
        let file_spdx_identifier = record.value_for_required_key(KEY_SPDXID)?.clone();
        let file_checksum: Vec<_> = record
            .values_for_key(KEY_FILECHECKSUM)
            .into_iter()
            .filter_map(|a| Checksum::try_from(a.as_ref()).ok().map(|newtype| newtype.0))
            .collect();
        if file_checksum.is_empty() {
            return Err(RecordError::MissingField(KEY_FILECHECKSUM.to_string()));
        }
        let concluded_license =
            models::SPDXExpression::parse(record.value_for_required_key(KEY_LICENSECONCLUDED)?)?;

        Ok(models::FileInformation {
            file_name,
            file_spdx_identifier,
            file_checksum,
            license_information_in_file: record
                .iter_values_for_key(KEY_LICENSEINFOINFILE)
                .cloned()
                .collect(),
            copyright_text: record
                .value_for_required_key(KEY_FILECOPYRIGHTTEXT)?
                .clone(),
            ..models::FileInformation::default()
        })
    }
}

impl From<chrono::ParseError> for RecordError {
    fn from(e: chrono::ParseError) -> Self {
        RecordError::Message(format!("DateTime parsing error: {}", e.to_string()).to_string())
    }
}

fn try_parse_creation_info(record: &Record) -> Result<models::CreationInfo, RecordError> {
    Ok(models::CreationInfo {
        creators: record.iter_values_for_key("Creator").cloned().collect(),
        created: DateTime::from_str(record.value_for_required_key("Created")?)?,
        creator_comment: record.value_for_key("CreatorComment")?.cloned(),

        // todo
        license_list_version: None,
    })
}

const RELATIONSHIP_REGEX_STRING: &str =
    r"(?P<id>SPDXRef-[a-zA-Z0-9]+) (?P<relationship>[-_a-z]+) (?P<relatedId>SPDXRef-[a-zA-Z0-9]+)";

struct CreationInformationAndRelationships {
    document_creation_information: models::DocumentCreationInformation,
    relationships: Vec<models::Relationship>,
}
fn captures_to_relationship(caps: &Captures) -> Option<models::Relationship> {
    let relationship_type = caps.name("relationship")?.as_str().to_uppercase();
    let d: BorrowedStrDeserializer<RecordError> = BorrowedStrDeserializer::new(&relationship_type);
    let relationship_type = models::RelationshipType::deserialize(d).ok()?;
    Some(models::Relationship {
        spdx_element_id: caps.name("id")?.as_str().to_owned(),
        related_spdx_element: caps.name("relatedId")?.as_str().to_owned(),
        relationship_type,
        comment: None,
    })
}

fn parse_relationships<'a>(
    strings: impl Iterator<Item = &'a String>,
) -> Result<Vec<models::Relationship>, RecordError> {
    let re = Regex::new(RELATIONSHIP_REGEX_STRING).unwrap();
    let result: Vec<_> = strings
        .filter_map(|s| {
            let caps = re.captures(s)?;
            captures_to_relationship(&caps)
        })
        .collect();
    Ok(result)
}

impl TryFrom<Record> for CreationInformationAndRelationships {
    type Error = RecordError;

    fn try_from(record: Record) -> Result<Self, Self::Error> {
        Ok(CreationInformationAndRelationships {
            document_creation_information: models::DocumentCreationInformation {
                spdx_version: record.value_for_required_key("SPDXVersion")?.clone(),
                data_license: record.value_for_required_key("DataLicense")?.clone(),
                spdx_identifier: record.value_for_required_key("SPDXID")?.clone(),
                document_name: record.value_for_required_key("DocumentName")?.clone(),
                spdx_document_namespace: record
                    .value_for_required_key("DocumentNamespace")?
                    .clone(),
                // todo
                external_document_references: vec![],
                creation_info: try_parse_creation_info(&record)?,
                document_comment: record.value_for_key("DocumentComment")?.cloned(),
                // todo
                document_describes: vec![],
            },
            relationships: parse_relationships(record.iter_values_for_key("Relationship"))?,
        })
    }
}
trait TryIntoSpdx: Iterator<Item = Record> {}

pub fn try_parse_spdx_doc_from_records<T: Iterator<Item = Record>>(
    mut records: T,
) -> Result<models::SPDX, RecordError> {
    let header = records.next().ok_or(RecordError::OutOfData)?;
    let creation_info_and_relationships = CreationInformationAndRelationships::try_from(header)?;
    let file_information: Vec<_> = records
        .filter_map(|record| models::FileInformation::try_from(record).ok())
        .collect();

    // todo handle packages
    Ok(models::SPDX {
        document_creation_information: creation_info_and_relationships
            .document_creation_information,
        package_information: vec![],
        other_licensing_information_detected: vec![],
        file_information,
        snippet_information: vec![],
        relationships: creation_info_and_relationships.relationships,
        annotations: vec![],
        spdx_ref_counter: 0,
    })
}

// fn parse_license_concluded(value: &str) -> Option<String> {
//     match value {
//         "NOASSERTION" => None,
//         _ => Some(value),
//     }
// }
// fn parse_checksum(value: &str) -> Option<Checksum> {
//     match ParsedLine::from(value) {
//         ParsedLine::RecordDelimeter => None,
//         ParsedLine::ValueOnly(_) => None,
//         ParsedLine::KVPair(pair) => {
//             if pair.key == "SHA1" {
//                 Some(Checksum::SHA1(pair.value.to_string()))
//             } else {
//                 None
//             }
//         }
//     }
// }
