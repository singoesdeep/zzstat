//! Condition parser for dynamic transform evaluation.
//!
//! Provides a JSON-serializable definition of logical conditions
//! that can be evaluated dynamically against a `StatContext`.

use crate::context::StatContext;
use serde::{Deserialize, Serialize};

/// Definition of a condition that can be evaluated against a `StatContext`.
///
/// Supports basic comparisons and logical combinations (And, Or, Not).
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "operator")]
pub enum ConditionDef {
    /// True if the value in context exactly matches the provided JSON value.
    Equals {
        key: String,
        value: serde_json::Value,
    },

    /// True if the value in context does not match, or is missing.
    NotEquals {
        key: String,
        value: serde_json::Value,
    },

    /// True if the numeric value in context is strictly greater.
    GreaterThan { key: String, value: f64 },

    /// True if the numeric value in context is strictly less.
    LessThan { key: String, value: f64 },

    /// True if all sub-conditions are true.
    And { conditions: Vec<ConditionDef> },

    /// True if any sub-condition is true.
    Or { conditions: Vec<ConditionDef> },

    /// True if the sub-condition is false.
    Not { condition: Box<ConditionDef> },
}

impl ConditionDef {
    /// Builds a thread-safe closure that evaluates this condition.
    pub fn build_closure(&self) -> Box<dyn Fn(&StatContext) -> bool + Send + Sync> {
        let def = self.clone();
        Box::new(move |ctx| def.evaluate(ctx))
    }

    /// Evaluates the condition directly against the context.
    fn evaluate(&self, ctx: &StatContext) -> bool {
        match self {
            ConditionDef::Equals { key, value } => {
                if let Some(ctx_val) = ctx.get::<serde_json::Value>(key) {
                    ctx_val == *value
                } else {
                    false
                }
            }
            ConditionDef::NotEquals { key, value } => {
                if let Some(ctx_val) = ctx.get::<serde_json::Value>(key) {
                    ctx_val != *value
                } else {
                    true
                }
            }
            ConditionDef::GreaterThan { key, value } => {
                if let Some(ctx_val) = ctx.get::<f64>(key) {
                    ctx_val > *value
                } else {
                    false
                }
            }
            ConditionDef::LessThan { key, value } => {
                if let Some(ctx_val) = ctx.get::<f64>(key) {
                    ctx_val < *value
                } else {
                    false
                }
            }
            ConditionDef::And { conditions } => conditions.iter().all(|c| c.evaluate(ctx)),
            ConditionDef::Or { conditions } => conditions.iter().any(|c| c.evaluate(ctx)),
            ConditionDef::Not { condition } => !condition.evaluate(ctx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_condition_evaluation() {
        let mut ctx = StatContext::new();
        ctx.set("is_undead", true);
        ctx.set("enemy_level", 50);

        let equals = ConditionDef::Equals {
            key: "is_undead".to_string(),
            value: serde_json::json!(true),
        };
        assert!(equals.evaluate(&ctx));

        let not_equals = ConditionDef::NotEquals {
            key: "is_undead".to_string(),
            value: serde_json::json!(false),
        };
        assert!(not_equals.evaluate(&ctx));

        let greater = ConditionDef::GreaterThan {
            key: "enemy_level".to_string(),
            value: 40.0,
        };
        assert!(greater.evaluate(&ctx));

        let less = ConditionDef::LessThan {
            key: "enemy_level".to_string(),
            value: 40.0,
        };
        assert!(!less.evaluate(&ctx));

        let and_cond = ConditionDef::And {
            conditions: vec![equals.clone(), greater.clone()],
        };
        assert!(and_cond.evaluate(&ctx));
    }
}
