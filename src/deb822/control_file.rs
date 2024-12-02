// Copyright 2021-2024, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Syntax for writing Debian control files
//!
//! See <https://www.debian.org/doc/debian-policy/ch-controlfields#s-controlsyntax>

use std::convert::TryFrom;

use thiserror;

/// Format the first line of a field: the name and an optional single line value.
fn format_field_first_line(
    field_name: &str,
    single_line_value: Option<&str>,
) -> Result<String, ControlFileError> {
    Ok(match single_line_value {
        Some(single_line_value) => format!("{}: {}", field_name.trim(), single_line_value.trim()),
        None => format!("{}:", field_name.trim()),
    })
}

/// Format a field, specifying the field name, optional single/first line value,
/// and optional iterator over subsequent value lines.
///
/// Used in implementing Field: Use one of the newtypes implementing Field
/// instead of calling this from outside the crate.
fn format_field<'a, T: Iterator<Item = &'a str>>(
    field_name: &str,
    single_line_value: Option<&str>,
    subsequent_lines: Option<T>,
) -> Result<String, ControlFileError> {
    let first_line = format_field_first_line(field_name, single_line_value)?;
    match subsequent_lines {
        Some(subsequent_lines) => {
            let lines: Vec<String> = vec![first_line]
                .into_iter()
                .chain(subsequent_lines.map(|line| {
                    let rest_of_line = if line.is_empty() {
                        "."
                    } else {
                        line.trim_end()
                    };
                    format!("  {rest_of_line}")
                }))
                .collect();
            Ok(lines.join("\n"))
        }
        None => Ok(first_line),
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ControlFileError {
    #[error("Unexpected newline in field {0}")]
    UnexpectedNewline(String),
    #[error("No value in field {0}. If seen on export, means missing .ok()")]
    NoValue(String),
    #[error("No value in field. If seen on export, means missing .ok()")]
    NoValueAnon,
}

/// A trait implemented for different types of Debian "control file" (aka deb822) fields.
pub trait Field {
    /// Convert to a string with no trailing newline.
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError>;
}

/// Newtype wrapping a single line field value: name and value on the same line.
#[derive(Debug, Clone)]
pub struct SingleLineField(pub String);

impl From<String> for SingleLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SingleLineField {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl TryFrom<&Option<String>> for SingleLineField {
    fn try_from(value: &Option<String>) -> Result<Self, Self::Error> {
        value
            .as_ref()
            .map(|v| SingleLineField::from(v.to_string()))
            .ok_or(ControlFileError::NoValueAnon)
    }

    type Error = ControlFileError;
}

impl Field for SingleLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if self.0.contains('\n') {
            return Err(ControlFileError::UnexpectedNewline(field_name.to_string()));
        }
        format_field_first_line(field_name, Some(&self.0)).map(Some)
    }
}

/// Newtype wrapping a multi-line field value: value may be multiple lines,
/// and starts on the same line as the name.
#[derive(Debug, Clone)]
pub struct MultilineField(pub String);

impl From<String> for MultilineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl Field for MultilineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        let mut lines = self.0.split('\n');
        let first = lines.next();
        format_field(field_name, first, Some(lines)).map(Some)
    }
}

/// Newtype wrapping a multi-line field value where the value starts on the line following the name.
#[derive(Debug, Clone)]
pub struct MultilineEmptyFirstLineField(pub String);

impl From<String> for MultilineEmptyFirstLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for MultilineEmptyFirstLineField {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

// impl From<&Vec<String>> for MultilineEmptyFirstLineField {
//     fn from(s: &Vec<String>) -> Self {
//         Self(s.join("\n"))
//     }
// }

impl TryFrom<&Vec<String>> for MultilineEmptyFirstLineField {
    fn try_from(value: &Vec<String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Err(ControlFileError::NoValueAnon);
        }
        Ok(Self(value.join("\n")))
    }

    type Error = ControlFileError;
}

impl Field for MultilineEmptyFirstLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        let lines = self.0.split('\n');
        format_field(field_name, None, Some(lines)).map(Some)
    }
}

/// Newtype wrapping a multi-line field value: value may be multiple lines,
/// and is on the same line as the name if single-line, but starts on the subsequent
/// line if multi-line.
#[derive(Debug, Clone)]
pub struct SingleLineOrMultilineEmptyFirstLineField(pub String);

impl From<String> for SingleLineOrMultilineEmptyFirstLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl Field for SingleLineOrMultilineEmptyFirstLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if self.0.contains('\n') {
            format_field(field_name, None, Some(self.0.split('\n')))
        } else {
            format_field_first_line(field_name, Some(&self.0))
        }
        .map(Some)
    }
}

/// An optional field is still a field
impl<F: Field> Field for Option<F> {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if let Some(field) = self {
            field.try_to_string(field_name)
        } else {
            Ok(None)
        }
    }
}

/// Trait to implement for various types of control file paragraphs.
///
/// Typically structures implementing this trait will hold one or more types implementing Field.
pub trait Paragraph {
    /// Convert a number of fields to a string with no trailing newline.
    ///
    /// Recommend implementing this with a call to ParagraphAccumulator::default(),
    /// with one or more chained `.write()?` calls, terminated by a call to the `to_string()` method.
    fn try_to_string(&self) -> Result<Option<String>, ControlFileError>;

    /// Convert a number of fields to a string with no trailing newline, dropping any errors
    fn try_to_string_ok(&self) -> Option<String> {
        self.try_to_string().ok().flatten()
    }
}

/// Trait providing features for iterators over paragraphs.
pub trait Paragraphs<'a>: 'a {
    /// Iterate over the strings for paragraphs that successfully converted to strings.
    fn flatten_to_strings(self) -> Box<dyn Iterator<Item = String> + 'a>;
}

impl<'a, T: Paragraph, U: 'a + Iterator<Item = T>> Paragraphs<'a> for U {
    fn flatten_to_strings(self) -> Box<dyn Iterator<Item = String> + 'a> {
        Box::new(self.filter_map(|paragraph| paragraph.try_to_string_ok()))
    }
}

/// Lets you incrementally serialize a paragraph, a field at a time.
///
/// Most useful in implementing Paragraph.
#[derive(Debug, Default)]
pub struct ParagraphAccumulator {
    field_lines: Vec<String>,
}

impl ParagraphAccumulator {
    /// Write a field to this paragraph, omitting it if its `try_to_string` method returned `Ok(None)`.
    pub fn write<F: Field>(
        mut self,
        field_name: &str,
        field: &F,
    ) -> Result<ParagraphAccumulator, ControlFileError> {
        if let Some(s) = field.try_to_string(field_name)? {
            self.field_lines.push(s);
        }
        Ok(self)
    }
}

impl ToString for ParagraphAccumulator {
    /// Return a string containing the whole paragraph, with no trailing newline.
    fn to_string(&self) -> String {
        self.field_lines.join("\n")
    }
}
