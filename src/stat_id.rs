//! Stat identifier module.
//!
//! Provides the `StatId` type, which is an interned string identifier
//! for stats. Uses `Arc<str>` for memory efficiency and fast comparison.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;

/// Interned string identifier for stats.
///
/// Uses `Arc<str>` for memory efficiency and fast comparison.
/// Multiple `StatId` instances with the same string content share the same
/// underlying allocation.
///
/// # Examples
///
/// ```rust
/// use zzstat::StatId;
///
/// let hp = StatId::from_str("HP");
/// let atk = StatId::from_str("ATK");
///
/// // Can be created from string slices or owned strings
/// let hp2: StatId = "HP".into();
/// let hp3: StatId = String::from("HP").into();
///
/// assert_eq!(hp, hp2);
/// assert_eq!(hp, hp3);
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatId(Arc<str>);

impl Serialize for StatId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.as_ref().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for StatId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(StatId::from(s))
    }
}

impl StatId {
    /// Create a new `StatId` from a string slice.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatId;
    ///
    /// let stat_id = StatId::from_str("HP");
    /// assert_eq!(stat_id.as_str(), "HP");
    /// ```
    pub fn from_str(s: &str) -> Self {
        Self(Arc::from(s))
    }

    /// Get the string representation of this `StatId`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatId;
    ///
    /// let stat_id = StatId::from_str("ATK");
    /// assert_eq!(stat_id.as_str(), "ATK");
    /// ```
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for StatId {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<String> for StatId {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

impl std::fmt::Display for StatId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_id_creation() {
        let id1 = StatId::from_str("HP");
        let id2 = StatId::from_str("HP");
        assert_eq!(id1, id2);
        assert_eq!(id1.as_str(), "HP");
    }

    #[test]
    fn test_stat_id_from_string() {
        let id: StatId = "ATK".into();
        assert_eq!(id.as_str(), "ATK");
    }

    #[test]
    fn test_stat_id_ordering() {
        let atk = StatId::from_str("ATK");
        let hp = StatId::from_str("HP");
        assert!(atk < hp); // "ATK" < "HP" lexicographically
    }
}

