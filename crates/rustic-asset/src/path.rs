//! Logical asset paths. See `PLAN.md` Section 6.
//!
//! `AssetPath` is a logical ID, not a filesystem path. It is normalized to
//! forward slashes, trimmed of leading/trailing slashes, and contains no
//! `..` segments. This is the same shape mods will use, so the rules are
//! tight.

use crate::error::{AssetError, AssetResult};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssetPath(String);

impl AssetPath {
    pub fn new(raw: impl Into<String>) -> AssetResult<Self> {
        let raw = raw.into();
        let normalized = normalize(&raw)?;
        Ok(Self(normalized))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }
}

fn normalize(raw: &str) -> AssetResult<String> {
    if raw.is_empty() {
        return Err(AssetError::InvalidPath("empty path".into()));
    }
    let unified = raw.replace('\\', "/");
    let mut out = String::with_capacity(unified.len());
    let mut first = true;
    for seg in unified.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            return Err(AssetError::InvalidPath(format!("'..' segment in {raw}")));
        }
        if !first {
            out.push('/');
        }
        out.push_str(seg);
        first = false;
    }
    if out.is_empty() {
        return Err(AssetError::InvalidPath(format!(
            "path normalizes to empty: {raw}"
        )));
    }
    Ok(out)
}

impl fmt::Display for AssetPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for AssetPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for AssetPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::new(raw).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_separators_and_dots() {
        let p = AssetPath::new("songs\\bopeebo/./inst.ogg").unwrap();
        assert_eq!(p.as_str(), "songs/bopeebo/inst.ogg");
    }

    #[test]
    fn trims_leading_and_trailing_slashes() {
        let p = AssetPath::new("/songs/tutorial/").unwrap();
        assert_eq!(p.as_str(), "songs/tutorial");
    }

    #[test]
    fn rejects_parent_segments() {
        assert!(AssetPath::new("songs/../etc/passwd").is_err());
    }

    #[test]
    fn rejects_empty() {
        assert!(AssetPath::new("").is_err());
        assert!(AssetPath::new("///").is_err());
    }

    #[test]
    fn serde_deserialize_normalizes_and_rejects_parent() {
        let p: AssetPath = serde_json::from_str(r#""images\\bf/./idle.png""#).unwrap();
        assert_eq!(p.as_str(), "images/bf/idle.png");

        let err = serde_json::from_str::<AssetPath>(r#""images/../secret.png""#);
        assert!(err.is_err());
    }
}
