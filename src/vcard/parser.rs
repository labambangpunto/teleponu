use std::fmt;

use super::model::{Address, StructuredName, TypedValue, VCard};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VCardError {
    EmptyFile,
    UnclosedVCard,
    MissingVersion { card_index: usize },
    UnsupportedVersion { card_index: usize, found: String },
    InvalidPropertyLine { line_number: usize, content: String },
}

impl fmt::Display for VCardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFile => write!(f, "The VCF file is empty or contains no VCARD blocks."),
            Self::UnclosedVCard => {
                write!(f, "A BEGIN:VCARD block is missing its closing END:VCARD.")
            }
            Self::MissingVersion { card_index } => {
                write!(f, "Contact #{} is missing the VERSION:3.0 property.", card_index)
            }
            Self::UnsupportedVersion { card_index, found } => write!(
                f,
                "Contact #{} uses version '{}'. Only VCF version 3.0 is supported.",
                card_index, found
            ),
            Self::InvalidPropertyLine {
                line_number,
                content,
            } => write!(
                f,
                "Invalid property syntax on line {}: '{}'",
                line_number, content
            ),
        }
    }
}

impl std::error::Error for VCardError {}

/// Unfolds folded lines per RFC 2426 (continuation lines starting with space/tab).
fn unfold_lines(input: &str) -> Vec<String> {
    let normalized = input.replace("\r\n", "\n");
    let mut unfolded: Vec<String> = Vec::new();

    for raw_line in normalized.split('\n') {
        if let Some(rest) = raw_line.strip_prefix(' ').or_else(|| raw_line.strip_prefix('\t')) {
            if let Some(last) = unfolded.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        unfolded.push(raw_line.to_string());
    }

    unfolded
}

/// Splits text by `sep` while ignoring escaped separators (`\;` or `\,`).
fn split_unescaped(input: &str, sep: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push('\\');
            current.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == sep {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }

    if escaped {
        current.push('\\');
    }
    parts.push(current);
    parts
}

fn unescape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }

    out
}

fn parse_types(params: &[&str]) -> Vec<String> {
    let mut types = Vec::new();
    for param in params {
        let p = param.trim();
        if p.is_empty() {
            continue;
        }
        if let Some((key, val)) = p.split_once('=') {
            if key.trim().eq_ignore_ascii_case("TYPE") {
                for t in val.split(',') {
                    let clean = t.trim().trim_matches('"').to_uppercase();
                    if !clean.is_empty() && !types.contains(&clean) {
                        types.push(clean);
                    }
                }
            }
        } else {
            let clean = p.trim_matches('"').to_uppercase();
            if !clean.is_empty() && !types.contains(&clean) {
                types.push(clean);
            }
        }
    }
    types
}

pub fn parse_vcf(input: &str) -> Result<Vec<VCard>, VCardError> {
    let lines = unfold_lines(input);
    let mut cards = Vec::new();
    let mut current: Option<VCard> = None;
    let mut has_valid_version = false;

    for (idx, raw_line) in lines.iter().enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }

        if line.eq_ignore_ascii_case("BEGIN:VCARD") {
            current = Some(VCard::default());
            has_valid_version = false;
            continue;
        }

        if line.eq_ignore_ascii_case("END:VCARD") {
            let card = current.take().ok_or(VCardError::InvalidPropertyLine {
                line_number: idx + 1,
                content: line.to_string(),
            })?;

            if !has_valid_version {
                return Err(VCardError::MissingVersion {
                    card_index: cards.len() + 1,
                });
            }

            cards.push(card);
            continue;
        }

        let Some(card) = current.as_mut() else {
            return Err(VCardError::InvalidPropertyLine {
                line_number: idx + 1,
                content: line.to_string(),
            });
        };

        let Some((left, raw_value)) = line.split_once(':') else {
            return Err(VCardError::InvalidPropertyLine {
                line_number: idx + 1,
                content: line.to_string(),
            });
        };

        let segments: Vec<&str> = left.split(';').collect();
        let raw_name = segments[0].trim();
        let prop_name = raw_name
            .rsplit_once('.')
            .map(|(_, name)| name)
            .unwrap_or(raw_name)
            .to_uppercase();
        let params = &segments[1..];

        match prop_name.as_str() {
            "VERSION" => {
                let ver = raw_value.trim();
                if ver != "3.0" {
                    return Err(VCardError::UnsupportedVersion {
                        card_index: cards.len() + 1,
                        found: ver.to_string(),
                    });
                }
                has_valid_version = true;
            }
            "N" => {
                let parts: Vec<String> = split_unescaped(raw_value, ';')
                    .into_iter()
                    .map(|p| unescape_text(&p))
                    .collect();
                card.n = StructuredName {
                    family: parts.first().cloned().unwrap_or_default(),
                    given: parts.get(1).cloned().unwrap_or_default(),
                    middle: parts.get(2).cloned().unwrap_or_default(),
                    prefix: parts.get(3).cloned().unwrap_or_default(),
                    suffix: parts.get(4).cloned().unwrap_or_default(),
                };
            }
            "FN" => {
                card.fn_name = unescape_text(raw_value);
            }
            "ORG" => {
                card.org = unescape_text(raw_value);
            }
            "TITLE" => {
                card.title = unescape_text(raw_value);
            }
            "TEL" => {
                card.phones.push(TypedValue {
                    types: parse_types(params),
                    value: unescape_text(raw_value),
                });
            }
            "EMAIL" => {
                card.emails.push(TypedValue {
                    types: parse_types(params),
                    value: unescape_text(raw_value),
                });
            }
            "URL" => {
                card.urls.push(unescape_text(raw_value));
            }
            "ADR" => {
                let parts: Vec<String> = split_unescaped(raw_value, ';')
                    .into_iter()
                    .map(|p| unescape_text(&p))
                    .collect();
                card.addresses.push(Address {
                    types: parse_types(params),
                    po_box: parts.first().cloned().unwrap_or_default(),
                    extended: parts.get(1).cloned().unwrap_or_default(),
                    street: parts.get(2).cloned().unwrap_or_default(),
                    city: parts.get(3).cloned().unwrap_or_default(),
                    region: parts.get(4).cloned().unwrap_or_default(),
                    postal_code: parts.get(5).cloned().unwrap_or_default(),
                    country: parts.get(6).cloned().unwrap_or_default(),
                });
            }
            "NOTE" => {
                card.note = unescape_text(raw_value);
            }
            _ => {
                card.extra_raw_lines.push(line.to_string());
            }
        }
    }

    if current.is_some() {
        return Err(VCardError::UnclosedVCard);
    }

    if cards.is_empty() {
        return Err(VCardError::EmptyFile);
    }

    Ok(cards)
}
