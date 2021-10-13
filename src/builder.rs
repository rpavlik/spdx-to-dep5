// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::str::FromStr;

use chrono::{DateTime, Utc};
use regex::{Captures, Regex};
use serde::{de, de::value::BorrowedStrDeserializer, Deserialize};
use spdx_rs::models;

use crate::tag_value::{KeyValuePair, ParsedLine};
/// An error from operations on a Record
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Found {1} fields named {0} instead of the zero or one expected.")]
    WantedAtMostOneFoundMore(String, usize),

    #[error("Found {1} fields named {0} instead of the one expected.")]
    WantedOneFoundMore(String, usize),

    #[error("Missing mandatory field {0}")]
    MissingField(String),

    #[error("Invalid value for {0} field")]
    InvalidField(String),

    #[error("Duplicated field {0}")]
    DuplicateField(String),

    #[error("SPDX-RS error {0}")]
    SpdxError(#[from] spdx_rs::error::SpdxError),

    #[error("Out of data")]
    OutOfData,

    #[error("Other error message: {0}")]
    Message(String),
}

impl de::Error for BuilderError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Message(msg.to_string())
    }
}

// impl TryFrom<&str> for Checksum {
//     type Error = ParserError;

//     fn try_from(value: &str) -> Result<Self, Self::Error> {
//         let pair = ParsedLine::from(value).pair().ok_or(ParserError::InvalidField(
//             "Could not parse checksum field".to_string(),
//         ))?;
//         let d: BorrowedStrDeserializer<Self::Error> = BorrowedStrDeserializer::new(&pair.key);
//         let algorithm: Algorithm = models::Algorithm::deserialize(d)?;
//         Ok(Checksum(models::Checksum {
//             algorithm,
//             value: pair.value,
//         }))
//     }
// }

fn try_parsing_checksum_from(
    field_name: &str,
    value: &str,
) -> Result<models::Checksum, BuilderError> {
    let pair = ParsedLine::from(value)
        .pair()
        .ok_or(BuilderError::InvalidField(field_name.to_string()))?;
    let d: BorrowedStrDeserializer<BuilderError> = BorrowedStrDeserializer::new(&pair.key);
    let algorithm = models::Algorithm::deserialize(d)?;
    Ok(models::Checksum {
        algorithm,
        value: pair.value,
    })
}

trait FieldReceiver {
    type Item;
    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError>;
    fn maybe_take(&mut self) -> Option<Self::Item>;
    fn has_required_fields(&self) -> bool;
}

fn set_single_multiplicity_string(
    dest: &mut Option<String>,
    field: &KeyValuePair,
) -> Result<bool, BuilderError> {
    if dest.is_some() {
        return Err(BuilderError::DuplicateField(field.key.clone()));
    }
    *dest = Some(field.value.to_string());
    Ok(true)
}
fn set_single_multiplicity_transformed<T, F>(
    dest: &mut Option<T>,
    field: &KeyValuePair,
    transformer: F,
) -> Result<bool, BuilderError>
where
    F: FnOnce(&KeyValuePair) -> Result<T, BuilderError>,
{
    if dest.is_some() {
        return Err(BuilderError::DuplicateField(field.key.clone()));
    }
    *dest = Some(transformer(field)?);
    Ok(true)
}
fn append_string(dest: &mut Vec<String>, field: &KeyValuePair) -> Result<bool, BuilderError> {
    dest.push(field.value.to_string());
    Ok(true)
}

fn append_transformed<T, F>(
    dest: &mut Vec<T>,
    field: &KeyValuePair,
    transformer: F,
) -> Result<bool, BuilderError>
where
    F: FnOnce(&KeyValuePair) -> Result<T, BuilderError>,
{
    dest.push(transformer(field)?);
    Ok(true)
}
#[derive(Debug, Default)]
struct CreationInformationBuilder {
    creator: Vec<String>,
    created: Option<DateTime<Utc>>,
    creator_comment: Option<String>,
}
impl FieldReceiver for CreationInformationBuilder {
    type Item = models::CreationInfo;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        match field.key.as_str() {
            "Creator" => append_string(&mut self.creator, field),
            "Created" => set_single_multiplicity_transformed(&mut self.created, field, |f| {
                Ok(DateTime::from_str(&f.value)?)
            }),
            "CreatorComment" => set_single_multiplicity_string(&mut self.creator_comment, field),
            _ => Ok(false),
        }
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        if !self.has_required_fields() {
            return None;
        }
        Some(models::CreationInfo {
            license_list_version: None,
            creators: std::mem::take(&mut self.creator),
            created: std::mem::take(&mut self.created)?,
            creator_comment: std::mem::take(&mut self.creator_comment),
        })
    }

    fn has_required_fields(&self) -> bool {
        !self.creator.is_empty() && self.created.is_some()
    }
}

#[derive(Debug, Default)]
struct DocumentCreationInformationBuilder {
    name: Option<String>,
    namespace: Option<String>,
    spdx_version: Option<String>,
    data_license: Option<String>,
    spdx_id: Option<String>,
    doc_comment: Option<String>,
    creation_info: CreationInformationBuilder,
}

impl FieldReceiver for DocumentCreationInformationBuilder {
    type Item = models::DocumentCreationInformation;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        match field.key.as_str() {
            "SPDXVersion" => set_single_multiplicity_string(&mut self.spdx_version, &field),
            "DataLicense" => set_single_multiplicity_string(&mut self.data_license, &field),
            "SPDXID" => {
                if self.spdx_id.is_none() {
                    set_single_multiplicity_string(&mut self.spdx_id, &field)
                } else {
                    // lots of things are named spdxid
                    Ok(false)
                }
            }
            "DocumentName" => set_single_multiplicity_string(&mut self.name, &field),
            "DocumentNamespace" => set_single_multiplicity_string(&mut self.namespace, &field),
            "DocumentComment" => set_single_multiplicity_string(&mut self.doc_comment, &field),
            _ => self.creation_info.maybe_handle_field(field),
        }
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        if !self.has_required_fields() {
            return None;
        }
        Some(models::DocumentCreationInformation {
            spdx_version: std::mem::take(&mut self.spdx_version)?,
            data_license: std::mem::take(&mut self.data_license)?,
            spdx_identifier: std::mem::take(&mut self.spdx_id)?,
            document_name: std::mem::take(&mut self.name)?,
            spdx_document_namespace: std::mem::take(&mut self.namespace)?,
            external_document_references: vec![],
            creation_info: self.creation_info.maybe_take()?,
            document_comment: std::mem::take(&mut self.doc_comment),
            document_describes: vec![],
        })
    }

    fn has_required_fields(&self) -> bool {
        self.spdx_version.is_some()
            && self.data_license.is_some()
            && self.spdx_id.is_some()
            && self.name.is_some()
            && self.namespace.is_some()
            && self.creation_info.has_required_fields()
    }
}

#[derive(Debug)]
struct RelationshipsBuilder {
    re: Regex,
    relationships: Vec<models::Relationship>,
}

impl RelationshipsBuilder {
    fn new() -> Self {
        Self {
            re: Regex::new(RELATIONSHIP_REGEX_STRING).unwrap(),
            relationships: vec![],
        }
    }
}
impl Default for RelationshipsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldReceiver for RelationshipsBuilder {
    type Item = Vec<models::Relationship>;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        if field.key == "Relationship" {
            let caps = self
                .re
                .captures(&field.value)
                .ok_or(BuilderError::InvalidField(field.key.to_string()))?;
            self.relationships.push(
                captures_to_relationship(&caps)
                    .ok_or(BuilderError::InvalidField(field.key.to_string()))?,
            );

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        Some(std::mem::take(&mut self.relationships))
    }

    fn has_required_fields(&self) -> bool {
        true
    }
}

#[derive(Debug, Default, PartialEq)]
struct FileInformationBuilder {
    file_name: Option<String>,
    file_spdx_identifier: Option<String>,
    file_type: Vec<models::FileType>,
    file_checksum: Vec<models::Checksum>,
    concluded_license: Option<models::SPDXExpression>,
    file_copyright_text: Option<String>,
    license_information_in_file: Vec<String>,
}

const KEY_FILENAME: &str = &"FileName";
const KEY_SPDXID: &str = &"SPDXID";
const KEY_FILECHECKSUM: &str = &"FileChecksum";
const KEY_LICENSECONCLUDED: &str = &"LicenseConcluded";
const KEY_LICENSEINFOINFILE: &str = &"LicenseInfoInFile";
const KEY_FILECOPYRIGHTTEXT: &str = &"FileCopyrightText";
impl FileInformationBuilder {
    fn is_known_field(key: &str) -> bool {
        match key {
            KEY_FILENAME => true,
            KEY_SPDXID => true,
            KEY_LICENSECONCLUDED => true,
            KEY_FILECOPYRIGHTTEXT => true,
            KEY_FILECHECKSUM => true,
            KEY_LICENSEINFOINFILE => true,
            _ => false,
        }
    }
    fn can_accept(&self, field: &KeyValuePair) -> bool {
        match field.key.as_str() {
            KEY_FILENAME => self.file_name.is_none(),
            KEY_SPDXID => self.file_spdx_identifier.is_none(),
            KEY_LICENSECONCLUDED => self.concluded_license.is_none(),
            KEY_FILECOPYRIGHTTEXT => self.file_copyright_text.is_none(),
            KEY_FILECHECKSUM => true,
            KEY_LICENSEINFOINFILE => true,
            _ => panic!("logic error"),
        }
    }
    fn is_empty(&self) -> bool {
        self.file_name.is_none()
            && self.file_spdx_identifier.is_none()
            && self.file_type.is_empty()
            && self.file_checksum.is_empty()
            && self.concluded_license.is_none()
            && self.file_copyright_text.is_none()
            && self.license_information_in_file.is_empty()
    }
}

impl FieldReceiver for FileInformationBuilder {
    type Item = models::FileInformation;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        match field.key.as_str() {
            KEY_FILENAME => set_single_multiplicity_string(&mut self.file_name, field),
            KEY_SPDXID => set_single_multiplicity_string(&mut self.file_spdx_identifier, field),
            KEY_LICENSECONCLUDED => {
                set_single_multiplicity_transformed(&mut self.concluded_license, field, |f| {
                    Ok(models::SPDXExpression::parse(&f.value)?)
                })
            }
            KEY_FILECOPYRIGHTTEXT => {
                set_single_multiplicity_string(&mut self.file_copyright_text, field)
            }
            KEY_FILECHECKSUM => append_transformed(&mut self.file_checksum, field, |f| {
                try_parsing_checksum_from(&f.key, &f.value)
            }),
            KEY_LICENSEINFOINFILE => append_string(&mut self.license_information_in_file, field),
            _ => panic!("logic error"),
        }
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        if !self.has_required_fields() {
            return None;
        }
        Some(models::FileInformation {
            file_name: std::mem::take(&mut self.file_name)?,
            file_spdx_identifier: std::mem::take(&mut self.file_spdx_identifier)?,
            file_type: std::mem::take(&mut self.file_type),
            file_checksum: std::mem::take(&mut self.file_checksum),
            concluded_license: std::mem::take(&mut self.concluded_license)?,
            license_information_in_file: std::mem::take(&mut self.license_information_in_file),
            comments_on_license: None,
            copyright_text: std::mem::take(&mut self.file_copyright_text)?,
            file_comment: None,
            file_notice: None,
            file_contributor: vec![],
            file_attribution_text: None,
        })
    }

    fn has_required_fields(&self) -> bool {
        self.file_name.is_some()
            && self.file_copyright_text.is_some()
            && !self.file_checksum.is_empty()
            && self.file_spdx_identifier.is_some()
    }
}

#[derive(Debug, Default)]
struct FileInformationCollectionBuilder {
    pending: FileInformationBuilder,
    file_info: Vec<models::FileInformation>,
}

impl FieldReceiver for FileInformationCollectionBuilder {
    type Item = Vec<models::FileInformation>;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        if !FileInformationBuilder::is_known_field(&field.key) {
            return Ok(false);
        }
        if !self.pending.can_accept(field) {
            if self.pending.has_required_fields() {
                self.file_info.push(self.pending.maybe_take().unwrap());
            } else {
                return Err(BuilderError::MissingField("something".to_string()));
            }
        }
        self.pending.maybe_handle_field(field)
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        if !self.has_required_fields() {
            return None;
        }
        if self.pending.has_required_fields() {
            self.file_info.push(self.pending.maybe_take()?);
        }
        Some(std::mem::take(&mut self.file_info))
    }

    fn has_required_fields(&self) -> bool {
        self.pending.is_empty() || self.pending.has_required_fields()
    }
}

impl From<chrono::ParseError> for BuilderError {
    fn from(e: chrono::ParseError) -> Self {
        BuilderError::Message(format!("DateTime parsing error: {}", e.to_string()).to_string())
    }
}

const RELATIONSHIP_REGEX_STRING: &str =
    r"(?P<id>SPDXRef-[a-zA-Z0-9]+) (?P<relationship>[-_a-z]+) (?P<relatedId>SPDXRef-[a-zA-Z0-9]+)";

fn captures_to_relationship(caps: &Captures) -> Option<models::Relationship> {
    let relationship_type = caps.name("relationship")?.as_str().to_uppercase();
    let d: BorrowedStrDeserializer<BuilderError> = BorrowedStrDeserializer::new(&relationship_type);
    let relationship_type = models::RelationshipType::deserialize(d).ok()?;
    Some(models::Relationship {
        spdx_element_id: caps.name("id")?.as_str().to_owned(),
        related_spdx_element: caps.name("relatedId")?.as_str().to_owned(),
        relationship_type,
        comment: None,
    })
}

#[derive(Debug, Default)]
pub struct SPDXBuilder {
    document_creation_information: DocumentCreationInformationBuilder,
    relationships: RelationshipsBuilder,
    file_collection: FileInformationCollectionBuilder,
}

impl SPDXBuilder {
    pub fn handle_field(&mut self, field: &KeyValuePair) -> Result<(), BuilderError> {
        self.maybe_handle_field(field)?;
        Ok(())
    }

    pub fn try_into_result(mut self) -> Option<models::SPDX> {
        self.maybe_take()
    }
}

impl FieldReceiver for SPDXBuilder {
    type Item = models::SPDX;

    fn maybe_handle_field(&mut self, field: &KeyValuePair) -> Result<bool, BuilderError> {
        Ok(self
            .document_creation_information
            .maybe_handle_field(field)?
            || self.relationships.maybe_handle_field(field)?
            || self.file_collection.maybe_handle_field(field)?)
    }

    fn maybe_take(&mut self) -> Option<Self::Item> {
        if self.has_required_fields() {
            Some(models::SPDX {
                document_creation_information: self.document_creation_information.maybe_take()?,
                package_information: vec![],
                other_licensing_information_detected: vec![],
                file_information: self.file_collection.maybe_take()?,
                snippet_information: vec![],
                relationships: self.relationships.maybe_take()?,
                annotations: vec![],
                spdx_ref_counter: 0,
            })
        } else {
            None
        }
    }

    fn has_required_fields(&self) -> bool {
        self.document_creation_information.has_required_fields()
            && self.relationships.has_required_fields()
            && self.file_collection.has_required_fields()
    }
}

trait ChainTryHandle: Sized {
    type Error;
    fn maybe_handle<T: FieldReceiver>(self, handler: &mut T) -> Result<Self, Self::Error>;
}

impl ChainTryHandle for Option<KeyValuePair> {
    type Error = BuilderError;
    fn maybe_handle<T: FieldReceiver>(self, handler: &mut T) -> Result<Self, Self::Error> {
        Ok(match self {
            Some(field) => {
                if handler.maybe_handle_field(&field)? {
                    None
                } else {
                    Some(field)
                }
            }
            None => None,
        })
    }
}
