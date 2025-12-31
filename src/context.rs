//! Context information for stat resolution.
//!
//! The `StatContext` provides a way to pass game state information
//! (combat state, zone type, difficulty, etc.) to sources and transforms
//! for conditional calculations. The core does not interpret this data;
//! it's simply passed through.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context information for stat resolution.
///
/// Contains combat state, target info, zone, difficulty, etc.
/// The core does not interpret this data - it's passed through to
/// sources and transforms for conditional calculations.
///
/// # Examples
///
/// ```rust
/// use zzstat::StatContext;
///
/// let mut context = StatContext::new();
/// context.set("in_combat", true);
/// context.set("zone_type", "pvp");
/// context.set("difficulty", 5);
///
/// let in_combat: Option<bool> = context.get("in_combat");
/// assert_eq!(in_combat, Some(true));
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatContext {
    /// Generic key-value pairs for context data.
    data: HashMap<String, serde_json::Value>,
}

impl StatContext {
    /// Create a new empty context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatContext;
    ///
    /// let context = StatContext::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a context value.
    ///
    /// The value must be serializable. If serialization fails, the value
    /// is silently not added.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatContext;
    ///
    /// let mut context = StatContext::new();
    /// context.set("in_combat", true);
    /// context.set("player_level", 50);
    /// context.set("zone_name", "Dungeon");
    /// ```
    pub fn set(&mut self, key: impl Into<String>, value: impl Serialize) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.into(), json_value);
        }
    }

    /// Get a context value.
    ///
    /// Returns `None` if the key doesn't exist or if the value
    /// cannot be deserialized to the requested type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatContext;
    ///
    /// let mut context = StatContext::new();
    /// context.set("difficulty", 5);
    ///
    /// let difficulty: Option<i32> = context.get("difficulty");
    /// assert_eq!(difficulty, Some(5));
    ///
    /// let missing: Option<i32> = context.get("missing");
    /// assert_eq!(missing, None);
    /// ```
    pub fn get<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data.get(key).and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Check if a key exists in the context.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatContext;
    ///
    /// let mut context = StatContext::new();
    /// context.set("key", "value");
    ///
    /// assert!(context.contains_key("key"));
    /// assert!(!context.contains_key("missing"));
    /// ```
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_set_get() {
        let mut ctx = StatContext::new();
        ctx.set("difficulty", 5);
        
        let difficulty: Option<i32> = ctx.get("difficulty");
        assert_eq!(difficulty, Some(5));
    }

    #[test]
    fn test_context_missing_key() {
        let ctx = StatContext::new();
        let value: Option<i32> = ctx.get("missing");
        assert_eq!(value, None);
    }
}

