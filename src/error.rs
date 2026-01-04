//! Error types for stat resolution.
//!
//! All errors that can occur during stat resolution are represented
//! by the `StatError` enum.

use crate::stat_id::StatId;
use thiserror::Error;

/// Format a cycle path as a readable string.
fn format_cycle_path(path: &[StatId]) -> String {
    if path.is_empty() {
        return String::from("(empty cycle)");
    }
    path.iter()
        .map(|id| id.as_str())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// Errors that can occur during stat resolution.
///
/// # Examples
///
/// ```rust
/// use zzstat::{StatError, StatId};
///
/// let err = StatError::MissingSource(StatId::from_str("HP"));
/// println!("{}", err); // "Missing source for stat: HP"
/// ```
#[derive(Debug, Error, Clone, PartialEq)]
pub enum StatError {
    /// A dependency cycle was detected in the stat graph.
    ///
    /// Contains the path of stats involved in the cycle.
    ///
    /// # Example
    ///
    /// If A depends on B, B depends on C, and C depends on A,
    /// this error will contain `[A, B, C, A]` showing the cycle.
    #[error("Cycle detected: {}", format_cycle_path(.path))]
    Cycle { path: Vec<StatId> },

    /// A required dependency stat was not found.
    ///
    /// This occurs when a transform declares a dependency on a stat
    /// that hasn't been resolved yet or doesn't exist.
    #[error("Missing dependency: {0}")]
    MissingDependency(StatId),

    /// No source was registered for a stat.
    ///
    /// This occurs when trying to resolve a stat that has no sources.
    #[error("Missing source for stat: {0}")]
    MissingSource(StatId),

    /// A transform application failed.
    ///
    /// Contains the stat ID and a description of what went wrong.
    #[error("Invalid transform for stat {0}: {1}")]
    InvalidTransform(StatId, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = StatError::MissingSource(StatId::from_str("HP"));
        assert!(err.to_string().contains("HP"));
    }

    #[test]
    fn test_cycle_error_display() {
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");
        let err = StatError::Cycle {
            path: vec![a.clone(), b.clone(), c.clone(), a.clone()],
        };
        let display = err.to_string();
        assert!(display.contains("Cycle detected"));
        assert!(display.contains("A"));
        assert!(display.contains("B"));
        assert!(display.contains("C"));
        assert!(display.contains(" -> "));
    }
}
