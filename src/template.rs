//! JSON Template Loader module.
//! 
//! Provides data structures for defining stats and transforms via JSON.
//! This allows for data-driven stat definitions instead of hardcoding them in Rust.

use crate::error::StatError;
use crate::resolver::StatResolver;
use crate::source::ConstantSource;
use crate::stat_id::StatId;
use crate::transform::{
    AdditiveTransform, ClampTransform, MultiplicativeTransform, ScalingTransform, StackRule,
    TransformPhase,
};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use crate::numeric::StatNumeric;

/// A template for a full set of stats and transforms.
///
/// Can be loaded from JSON and converted into a `StatResolver`.
///
/// # Examples
///
/// ```rust
/// use zzstat::template::StatTemplate;
///
/// let json = r#"{
///     "name": "Warrior",
///     "base_stats": {
///         "HP": 100.0,
///         "STR": 20.0
///     },
///     "transforms": [
///         {
///             "stat": "ATK",
///             "phase": "additive",
///             "type": "Scaling",
///             "depends_on": "STR",
///             "factor": 2.0
///         }
///     ]
/// }"#;
///
/// let template: StatTemplate = serde_json::from_str(json).unwrap();
/// let resolver = template.build_resolver().unwrap();
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatTemplate {
    /// The name of this template (e.g. "Warrior Class")
    pub name: String,

    /// Base stats. E.g. {"HP": 100.0, "STR": 20.0}
    #[serde(default)]
    pub base_stats: HashMap<String, f64>,

    /// Transforms and formulas for this template
    #[serde(default)]
    pub transforms: Vec<TransformTemplate>,
}

/// A template for a single transform.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransformTemplate {
    pub stat: String,
    pub phase: TransformPhaseDef,
    #[serde(default)]
    pub rule: Option<StackRuleDef>,
    #[serde(flatten)]
    pub transform_type: TransformTypeDef,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum TransformTypeDef {
    Additive {
        value: f64,
    },
    Multiplicative {
        value: f64,
    },
    Scaling {
        depends_on: String,
        factor: f64,
    },
    Clamp {
        min: Option<f64>,
        max: Option<f64>,
    },
    Conditional {
        condition: crate::condition::ConditionDef,
        transform: Box<TransformTypeDef>,
    },
}

impl TransformTypeDef {
    /// Builds the inner transform recursively.
    pub fn build_transform(&self) -> Box<dyn crate::transform::StatTransform> {
        match self {
            TransformTypeDef::Additive { value } => Box::new(AdditiveTransform::new(*value)),
            TransformTypeDef::Multiplicative { value } => {
                Box::new(MultiplicativeTransform::new(*value))
            }
            TransformTypeDef::Scaling { depends_on, factor } => {
                let dep_id = StatId::from(depends_on.as_str());
                Box::new(ScalingTransform::new(dep_id, *factor))
            }
            TransformTypeDef::Clamp { min, max } => {
                let min_val = min.map(crate::numeric::StatValue::from_f64);
                let max_val = max.map(crate::numeric::StatValue::from_f64);
                Box::new(ClampTransform::with_bounds(min_val, max_val))
            }
            TransformTypeDef::Conditional {
                condition,
                transform,
            } => {
                let inner = transform.build_transform();
                let closure = condition.build_closure();
                Box::new(crate::transform::ConditionalTransform::new(
                    closure,
                    inner,
                    "conditional transform", // Generic description
                ))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransformPhaseDef {
    Additive,
    Multiplicative,
    Final,
    Custom(u8),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum StackRuleDef {
    Override,
    Additive,
    Multiplicative,
    Diminishing { k: f64 },
    Min,
    Max,
    MinMax,
}

impl StatTemplate {
    /// Builds a `StatResolver` from this template.
    ///
    /// The resulting resolver will contain all the base stats as `ConstantSource`s
    /// and all the defined transforms.
    pub fn build_resolver(&self) -> Result<StatResolver, StatError> {
        let mut resolver = StatResolver::new();

        // 1. Register base stats
        for (stat_name, value) in &self.base_stats {
            let stat_id = StatId::from(stat_name.as_str());
            resolver.register_source(stat_id, Box::new(ConstantSource(*value)));
        }

        // 2. Register transforms
        for t in &self.transforms {
            let stat_id = StatId::from(t.stat.as_str());
            
            let phase = match t.phase {
                TransformPhaseDef::Additive => TransformPhase::Additive,
                TransformPhaseDef::Multiplicative => TransformPhase::Multiplicative,
                TransformPhaseDef::Final => TransformPhase::Final,
                TransformPhaseDef::Custom(n) => TransformPhase::Custom(n),
            };

            // Instantiate the correct transform type using the helper method
            let transform = t.transform_type.build_transform();

            match &t.rule {
                Some(r) => {
                    let rule = match r {
                        StackRuleDef::Override => StackRule::Override,
                        StackRuleDef::Additive => StackRule::Additive,
                        StackRuleDef::Multiplicative => StackRule::Multiplicative,
                        StackRuleDef::Diminishing { k } => StackRule::Diminishing {
                            k: crate::numeric::StatValue::from_f64(*k),
                        },
                        StackRuleDef::Min => StackRule::Min,
                        StackRuleDef::Max => StackRule::Max,
                        StackRuleDef::MinMax => StackRule::MinMax,
                    };
                    resolver.register_transform_with_rule(stat_id, phase, rule, transform);
                }
                None => {
                    // Let the resolver infer the rule from the transform/phase
                    resolver.register_transform_in_phase(stat_id, phase, transform);
                }
            }
        }

        Ok(resolver)
    }
}

#[cfg(test)]
mod tests {
    use crate::numeric::StatNumeric;
    use super::*;
    use crate::context::StatContext;

    #[test]
    fn test_template_deserialization_and_resolve() {
        let json = r#"{
            "name": "Warrior",
            "base_stats": {
                "STR": 20.0,
                "VIT": 15.0,
                "ATK": 0.0,
                "HP": 100.0
            },
            "transforms": [
                {
                    "stat": "ATK",
                    "phase": "additive",
                    "type": "Scaling",
                    "depends_on": "STR",
                    "factor": 2.0
                },
                {
                    "stat": "HP",
                    "phase": "additive",
                    "type": "Scaling",
                    "depends_on": "VIT",
                    "factor": 10.0
                },
                {
                    "stat": "ATK",
                    "phase": "final",
                    "rule": "minmax",
                    "type": "Clamp",
                    "max": 50.0
                }
            ]
        }"#;

        let template: StatTemplate = serde_json::from_str(json).unwrap();
        assert_eq!(template.name, "Warrior");
        assert_eq!(template.base_stats.len(), 4);
        assert_eq!(template.transforms.len(), 3);

        let mut resolver = template.build_resolver().unwrap();
        let context = StatContext::new();

        let atk = resolver.resolve(&StatId::from("ATK"), &context).unwrap();
        // Base ATK is 0, STR is 20, Scaling is 2.0. So 20 * 2 = 40.
        assert_eq!(atk.value.to_f64(), 40.0);

        let hp = resolver.resolve(&StatId::from("HP"), &context).unwrap();
        // Base HP is 100, VIT is 15, Scaling is 10.0. So 100 + 15 * 10 = 250.
        assert_eq!(hp.value.to_f64(), 250.0);
    }

    #[test]
    fn test_conditional_transform_deserialization() {
        let json = r#"{
            "name": "Conditional",
            "base_stats": {
                "ATK": 100.0
            },
            "transforms": [
                {
                    "stat": "ATK",
                    "phase": "multiplicative",
                    "type": "Conditional",
                    "condition": {
                        "operator": "Equals",
                        "key": "in_combat",
                        "value": true
                    },
                    "transform": {
                        "type": "Multiplicative",
                        "value": 1.5
                    }
                }
            ]
        }"#;

        let template: StatTemplate = serde_json::from_str(json).unwrap();
        let mut resolver = template.build_resolver().unwrap();
        
        let mut context = StatContext::new();
        let stat_id = StatId::from("ATK");

        // Not in combat, so ATK is 100
        let atk1 = resolver.resolve(&stat_id, &context).unwrap();
        assert_eq!(atk1.value.to_f64(), 100.0);

        // In combat, ATK gets x1.5 multiplier
        context.set("in_combat", true);
        resolver.invalidate_all();
        let atk2 = resolver.resolve(&stat_id, &context).unwrap();
        assert_eq!(atk2.value.to_f64(), 150.0);
    }
}
