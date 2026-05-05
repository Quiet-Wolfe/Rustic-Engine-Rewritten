//! Parser for base FNF `CoolUtil.coolTextFile` lists.
//!
//! ref: 50fccded:source/CoolUtil.hx:9-18

use crate::error::{AssetError, AssetResult};

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextList {
    pub items: Vec<String>,
}

impl TextList {
    pub fn parse(bytes: &[u8]) -> AssetResult<Self> {
        let raw = std::str::from_utf8(bytes)
            .map_err(|e| AssetError::InvalidData(format!("text list utf8: {e}")))?;
        let trimmed = raw.trim();
        let items = trimmed
            .split('\n')
            .map(|line| line.trim().to_string())
            .collect();
        Ok(Self { items })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn parses_like_cool_text_file() {
        let list = TextList::parse(b" Tutorial \r\nBopeebo\nFresh\nDadbattle\n\n").unwrap();
        assert_eq!(
            list.items,
            vec!["Tutorial", "Bopeebo", "Fresh", "Dadbattle"]
        );
    }

    #[test]
    fn preserves_inner_empty_lines() {
        let list = TextList::parse(b"A\n\nB\n").unwrap();
        assert_eq!(list.items, vec!["A", "", "B"]);
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert!(TextList::parse(&[0xff]).is_err());
    }
}
