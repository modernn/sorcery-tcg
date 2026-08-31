//! Canonical engine identity compatible with the existing TypeScript contract.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// A SHA-256 identity over canonical JSON bytes.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct IdentityHash(String);

impl fmt::Display for IdentityHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

/// A value could not be represented by the engine's canonical identity format.
#[derive(Debug)]
pub enum CanonicalError {
    /// Engine manifests, states, actions, and events permit integers only.
    NonIntegralNumber,
    /// JSON string serialization failed.
    StringSerialization(serde_json::Error),
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonIntegralNumber => {
                formatter.write_str("canonical engine JSON requires integers")
            }
            Self::StringSerialization(error) => error.fmt(formatter),
        }
    }
}

impl Error for CanonicalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NonIntegralNumber => None,
            Self::StringSerialization(error) => Some(error),
        }
    }
}

fn compare_utf16(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn write_value(value: &Value, output: &mut String) -> Result<(), CanonicalError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) if value.is_i64() || value.is_u64() => {
            output.push_str(&value.to_string());
        }
        Value::Number(_) => return Err(CanonicalError::NonIntegralNumber),
        Value::String(value) => output
            .push_str(&serde_json::to_string(value).map_err(CanonicalError::StringSerialization)?),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output)?;
            }
            output.push(']');
        }
        Value::Object(values) => {
            let mut entries: Vec<_> = values.iter().collect();
            entries.sort_unstable_by(|(left, _), (right, _)| compare_utf16(left, right));
            output.push('{');
            for (index, (key, value)) in entries.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).map_err(CanonicalError::StringSerialization)?,
                );
                output.push(':');
                write_value(value, output)?;
            }
            output.push('}');
        }
    }
    Ok(())
}

/// Serializes an engine value with UTF-16 key ordering and no insignificant whitespace.
///
/// # Errors
///
/// Returns [`CanonicalError`] for non-integral numbers or invalid strings.
pub fn canonical_json(value: &Value) -> Result<String, CanonicalError> {
    let mut output = String::new();
    write_value(value, &mut output)?;
    Ok(output)
}

/// Hashes an engine value using its canonical JSON bytes.
///
/// # Errors
///
/// Returns [`CanonicalError`] when the value cannot be canonicalized.
pub fn identity_hash(value: &Value) -> Result<IdentityHash, CanonicalError> {
    let bytes = canonical_json(value)?;
    let digest = Sha256::digest(bytes.as_bytes());
    let mut hash = String::with_capacity(71);
    hash.push_str("sha256:");
    for byte in digest {
        hash.push(HEX[usize::from(byte >> 4)] as char);
        hash.push(HEX[usize::from(byte & 0x0f)] as char);
    }
    Ok(IdentityHash(hash))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonical_json, identity_hash};

    #[test]
    fn canonical_json_should_match_existing_engine_state_vector() {
        let value = json!({
            "stateVersion": 0,
            "schemaVersion": 1,
            "prng": { "word": 0, "draws": 0, "algorithm": "mulberry32-v1" }
        });

        assert_eq!(
            canonical_json(&value).expect("canonical engine state"),
            r#"{"prng":{"algorithm":"mulberry32-v1","draws":0,"word":0},"schemaVersion":1,"stateVersion":0}"#
        );
    }

    #[test]
    fn identity_hash_should_match_existing_engine_state_vector() {
        let value = json!({
            "stateVersion": 0,
            "schemaVersion": 1,
            "prng": { "word": 0, "draws": 0, "algorithm": "mulberry32-v1" }
        });

        assert_eq!(
            identity_hash(&value)
                .expect("hash engine state")
                .to_string(),
            "sha256:9ec0a8e3f08e46c6c6e2a44542de0a7c20a50a1fe6af9ea5b195dd0379d4e4b2"
        );
    }
}
