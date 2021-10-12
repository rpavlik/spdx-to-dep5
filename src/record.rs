// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::key_value_parser::KeyValuePair;

/// An error from operations on a Record
#[derive(Debug, thiserror::Error)]
pub enum RecordError {
    #[error("Found {1} fields named {0} instead of the zero or one expected.")]
    WantedAtMostOneFoundMore(String, usize),

    #[error("Found {1} fields named {0} instead of the one expected.")]
    WantedOneFoundMore(String, usize),

    #[error("Missing mandatory field {0}")]
    MissingField(String),
}

/// An order collection of key-value pairs with no (unescaped) blank lines between.
pub struct Record(Vec<KeyValuePair>);

impl Default for Record {
    fn default() -> Self {
        Self(Vec::default())
    }
}

impl Record {
    pub fn push_field(&mut self, pair: KeyValuePair) {
        self.0.push(pair)
    }

    /// Return the number of fields whose key matches the provided key
    pub fn count_fields_with_key(&self, key: &str) -> usize {
        self.0.iter().filter(|pair| pair.key == key).count()
    }

    fn iter_values_for_key<'a>(
        &'a self,
        key: &'a str,
    ) -> Box<dyn Iterator<Item = &'a String> + 'a> {
        Box::new(self.0.iter().filter_map(move |pair| {
            if pair.key == key {
                Some(&pair.value)
            } else {
                None
            }
        }))
    }

    /// Return a vector of all field values (in original order) whose key matches the provided key
    pub fn values_for_key<'a>(&'a self, key: &'a str) -> Vec<&'a String> {
        self.iter_values_for_key(key).collect()
    }

    /// Returns the value of a field with the given key, if any, and returns an error if more than one such field exists.
    pub fn value_for_key<'a>(&'a self, key: &'a str) -> Result<Option<&'a String>, RecordError> {
        let mut values = self.iter_values_for_key(key);
        let value = values.next();
        if values.next().is_none() {
            Ok(value)
        } else {
            Err(RecordError::WantedAtMostOneFoundMore(
                key.to_string(),
                2 + values.count(),
            ))
        }
    }
    /// Returns the value of a field with the given key, and returns an error if more than one such field exists, or if none exist.
    pub fn value_for_required_key<'a>(&'a self, key: &'a str) -> Result<&'a String, RecordError> {
        let mut values = self.iter_values_for_key(key);
        match values.next() {
            Some(value) => {
                if values.next().is_none() {
                    Ok(value)
                } else {
                    Err(RecordError::WantedOneFoundMore(
                        key.to_string(),
                        2 + values.count(),
                    ))
                }
            }
            None => Err(RecordError::MissingField(key.to_string())),
        }
    }
}
