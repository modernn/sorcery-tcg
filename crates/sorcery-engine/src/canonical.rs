//! Canonical engine identity compatible with the existing TypeScript contract.

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

use std::collections::BTreeSet;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const HEX: &[u8; 16] = b"0123456789abcdef";

/// A SHA-256 identity over canonical JSON bytes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct IdentityHash(String);

impl IdentityHash {
    /// Parses a lowercase SHA-256 engine identity.
    ///
    /// # Errors
    ///
    /// Returns [`IdentityHashError`] when `value` is not `sha256:` followed by
    /// exactly 64 lowercase hexadecimal digits.
    pub fn parse(value: &str) -> Result<Self, IdentityHashError> {
        let Some(digest) = value.strip_prefix("sha256:") else {
            return Err(IdentityHashError);
        };
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(IdentityHashError);
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the identity string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for IdentityHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(de::Error::custom)
    }
}

/// An engine identity did not use the frozen lowercase SHA-256 syntax.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityHashError;

impl fmt::Display for IdentityHashError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("identity must be sha256: followed by 64 lowercase hexadecimal digits")
    }
}

impl Error for IdentityHashError {}

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

/// Parses JSON while rejecting duplicate object keys at every depth.
///
/// # Errors
///
/// Returns [`serde_json::Error`] for malformed JSON or duplicate object keys.
pub fn parse_json_without_duplicate_keys(text: &str) -> Result<Value, serde_json::Error> {
    struct CheckedValue(Value);

    impl<'de> Deserialize<'de> for CheckedValue {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct CheckedValueVisitor;

            impl<'de> Visitor<'de> for CheckedValueVisitor {
                type Value = CheckedValue;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("a JSON value without duplicate object keys")
                }

                fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::Bool(value)))
                }

                fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::Number(value.into())))
                }

                fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::Number(value.into())))
                }

                fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    serde_json::Number::from_f64(value)
                        .map(Value::Number)
                        .map(CheckedValue)
                        .ok_or_else(|| E::custom("non-finite JSON number"))
                }

                fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    self.visit_string(value.to_owned())
                }

                fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::String(value)))
                }

                fn visit_none<E>(self) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::Null))
                }

                fn visit_unit<E>(self) -> Result<Self::Value, E> {
                    Ok(CheckedValue(Value::Null))
                }

                fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
                where
                    A: SeqAccess<'de>,
                {
                    let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0));
                    while let Some(CheckedValue(value)) = sequence.next_element()? {
                        values.push(value);
                    }
                    Ok(CheckedValue(Value::Array(values)))
                }

                fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut keys = BTreeSet::new();
                    let mut values = serde_json::Map::new();
                    while let Some(key) = map.next_key::<String>()? {
                        if !keys.insert(key.clone()) {
                            return Err(de::Error::custom(format!("duplicate_key:{key}")));
                        }
                        let CheckedValue(value) = map.next_value()?;
                        values.insert(key, value);
                    }
                    Ok(CheckedValue(Value::Object(values)))
                }
            }

            deserializer.deserialize_any(CheckedValueVisitor)
        }
    }

    serde_json::from_str::<CheckedValue>(text).map(|CheckedValue(value)| value)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{canonical_json, identity_hash, parse_json_without_duplicate_keys};

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

    #[test]
    fn identity_hash_deserialization_should_reject_noncanonical_syntax() {
        let result = serde_json::from_str::<super::IdentityHash>(
            r#""SHA256:9EC0A8E3F08E46C6C6E2A44542DE0A7C20A50A1FE6AF9EA5B195DD0379D4E4B2""#,
        );

        assert!(result.is_err());
    }

    #[test]
    fn checked_json_should_reject_nested_duplicate_keys() {
        let error = parse_json_without_duplicate_keys(r#"{"outer":{"same":1,"same":2}}"#)
            .expect_err("duplicate key must be rejected");

        assert!(error.to_string().contains("duplicate_key:same"));
    }
}
