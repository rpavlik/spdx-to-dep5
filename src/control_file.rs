// Copyright 2021, Collabora, Ltd.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Syntax for writing Debian control files
//!
//! See <https://www.debian.org/doc/debian-policy/ch-controlfields#s-controlsyntax>

use thiserror;
pub fn format_field_first_line(
    field_name: &str,
    single_line_value: Option<&str>,
) -> Result<String, ControlFileError> {
    Ok(match single_line_value {
        Some(single_line_value) => format!("{}: {}", field_name.trim(), single_line_value.trim()),
        None => format!("{}:", field_name.trim()),
    })
}

pub fn format_field<'a, T: Iterator<Item = &'a str>>(
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
                    format!("  {}", rest_of_line)
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
}
pub trait Field {
    /// Convert to a string with no trailing newline.
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError>;
}

#[derive(Debug, Clone)]
pub struct SingleLineField(pub String);

impl From<String> for SingleLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl Field for SingleLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if self.0.contains("\n") {
            return Err(ControlFileError::UnexpectedNewline(field_name.to_string()));
        }
        format_field_first_line(field_name, Some(&self.0)).map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct MultilineField(pub String);

impl From<String> for MultilineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl Field for MultilineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        let mut lines = self.0.split("\n");
        let first = lines.next();
        format_field(field_name, first, Some(lines)).map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct MultilineEmptyFirstLineField(pub String);

impl From<String> for MultilineEmptyFirstLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl Field for MultilineEmptyFirstLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        let lines = self.0.split("\n");
        format_field(field_name, None, Some(lines)).map(Some)
    }
}

#[derive(Debug, Clone)]
pub struct SingleLineOrMultilineEmptyFirstLineField(pub String);

impl From<String> for SingleLineOrMultilineEmptyFirstLineField {
    fn from(s: String) -> Self {
        Self(s)
    }
}
impl Field for SingleLineOrMultilineEmptyFirstLineField {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if self.0.contains("\n") {
            format_field(field_name, None, Some(self.0.split("\n")))
        } else {
            format_field_first_line(field_name, Some(&self.0))
        }
        .map(Some)
    }
}

impl<F: Field> Field for Option<F> {
    fn try_to_string(&self, field_name: &str) -> Result<Option<String>, ControlFileError> {
        if let Some(field) = self {
            field.try_to_string(field_name)
        } else {
            Ok(None)
        }
    }
}

pub trait Paragraph {
    /// Convert a number of fields to a string with no trailing newline.
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

#[derive(Debug, Default)]
pub struct ParagraphAccumulator {
    field_lines: Vec<String>,
}

impl ParagraphAccumulator {
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
    fn to_string(&self) -> String {
        self.field_lines.join("\n")
    }
}
