//! Savaş Motoru (Combat Engine) Modülü.
//!
//! "Saldırgan" ve "Savunan" arasındaki matematiksel formülleri ağaç yapısında
//! değerlendirerek hasar ve etki hesaplamaları yapar. Rastgelelik içerdiğinde
//! dışarıdan verilen rastgele sayı üreticisine (RNG) dayanır, böylece deterministik kalır.

use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::StatNumeric;
use crate::resolver::StatResolver;
use crate::stat_id::StatId;
use serde::{Deserialize, Serialize};

/// Savaş sırasında okunacak verinin kaynağı.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CombatTarget {
    Attacker,
    Defender,
}

/// Savaş formülü ağacının parçaları.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum CombatExpression {
    /// Bir stat değerini okur.
    Stat { target: CombatTarget, stat: String },
    /// Sabit bir değer.
    Constant { value: f64 },
    /// İki ifadeyi toplar.
    Add {
        left: Box<CombatExpression>,
        right: Box<CombatExpression>,
    },
    /// İki ifadeyi çıkarır (left - right).
    Subtract {
        left: Box<CombatExpression>,
        right: Box<CombatExpression>,
    },
    /// İki ifadeyi çarpar.
    Multiply {
        left: Box<CombatExpression>,
        right: Box<CombatExpression>,
    },
    /// Değeri sınırlandırır.
    Clamp {
        min: Option<f64>,
        max: Option<f64>,
        expr: Box<CombatExpression>,
    },
    /// Rastgelelik barındıran ihtimal bloğu.
    Chance {
        /// Başarı ihtimalini belirten stat (Örn: CRIT_CHANCE, 0.0 ile 1.0 arası)
        chance_expr: Box<CombatExpression>,
        /// İhtimal başarılı olursa hesaplanacak ifade
        success_expr: Box<CombatExpression>,
        /// İhtimal başarısız olursa hesaplanacak ifade
        fail_expr: Box<CombatExpression>,
    },
}

/// İsimlendirilmiş savaş formülü.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CombatFormula {
    pub name: String,
    pub expression: CombatExpression,
}

/// Savaş motoru. İki tarafın durumunu alıp sonucu hesaplar.
pub struct CombatEngine;

impl CombatEngine {
    /// Savaş formülünü hesaplar.
    ///
    /// `rng` parametresi 0.0 ile 1.0 arasında rastgele sayı üreten bir fonksiyondur.
    /// Deterministic testler için sabit dönebilen bir closure verilebilir.
    pub fn evaluate<R>(
        formula: &CombatFormula,
        attacker_resolver: &mut StatResolver,
        attacker_ctx: &StatContext,
        defender_resolver: &mut StatResolver,
        defender_ctx: &StatContext,
        rng: &mut R,
    ) -> Result<f64, StatError>
    where
        R: FnMut() -> f64,
    {
        Self::eval_expr(
            &formula.expression,
            attacker_resolver,
            attacker_ctx,
            defender_resolver,
            defender_ctx,
            rng,
        )
    }

    fn eval_expr<R>(
        expr: &CombatExpression,
        attacker_resolver: &mut StatResolver,
        attacker_ctx: &StatContext,
        defender_resolver: &mut StatResolver,
        defender_ctx: &StatContext,
        rng: &mut R,
    ) -> Result<f64, StatError>
    where
        R: FnMut() -> f64,
    {
        match expr {
            CombatExpression::Stat { target, stat } => {
                let stat_id = StatId::from(stat.as_str());
                let val = match target {
                    CombatTarget::Attacker => attacker_resolver.resolve(&stat_id, attacker_ctx)?,
                    CombatTarget::Defender => defender_resolver.resolve(&stat_id, defender_ctx)?,
                };
                Ok(val.value.to_f64())
            }
            CombatExpression::Constant { value } => Ok(*value),
            CombatExpression::Add { left, right } => {
                let l = Self::eval_expr(
                    left,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                let r = Self::eval_expr(
                    right,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                Ok(l + r)
            }
            CombatExpression::Subtract { left, right } => {
                let l = Self::eval_expr(
                    left,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                let r = Self::eval_expr(
                    right,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                Ok(l - r)
            }
            CombatExpression::Multiply { left, right } => {
                let l = Self::eval_expr(
                    left,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                let r = Self::eval_expr(
                    right,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                Ok(l * r)
            }
            CombatExpression::Clamp { min, max, expr } => {
                let mut val = Self::eval_expr(
                    expr,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                if let Some(m) = min {
                    val = val.max(*m);
                }
                if let Some(m) = max {
                    val = val.min(*m);
                }
                Ok(val)
            }
            CombatExpression::Chance {
                chance_expr,
                success_expr,
                fail_expr,
            } => {
                let chance = Self::eval_expr(
                    chance_expr,
                    attacker_resolver,
                    attacker_ctx,
                    defender_resolver,
                    defender_ctx,
                    rng,
                )?;
                let roll = rng();
                if roll <= chance {
                    Self::eval_expr(
                        success_expr,
                        attacker_resolver,
                        attacker_ctx,
                        defender_resolver,
                        defender_ctx,
                        rng,
                    )
                } else {
                    Self::eval_expr(
                        fail_expr,
                        attacker_resolver,
                        attacker_ctx,
                        defender_resolver,
                        defender_ctx,
                        rng,
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ConstantSource;

    #[test]
    fn test_combat_engine_basic_damage() {
        let mut attacker = StatResolver::new();
        attacker.register_source(StatId::from("ATK"), Box::new(ConstantSource(150.0)));

        let mut defender = StatResolver::new();
        defender.register_source(StatId::from("DEF"), Box::new(ConstantSource(50.0)));

        let ctx = StatContext::new();

        let formula = CombatFormula {
            name: "Basic Attack".to_string(),
            expression: CombatExpression::Clamp {
                min: Some(0.0),
                max: None,
                expr: Box::new(CombatExpression::Subtract {
                    left: Box::new(CombatExpression::Stat {
                        target: CombatTarget::Attacker,
                        stat: "ATK".to_string(),
                    }),
                    right: Box::new(CombatExpression::Stat {
                        target: CombatTarget::Defender,
                        stat: "DEF".to_string(),
                    }),
                }),
            },
        };

        let damage = CombatEngine::evaluate(
            &formula,
            &mut attacker,
            &ctx,
            &mut defender,
            &ctx,
            &mut || 0.0, // RNG not used
        )
        .unwrap();

        assert_eq!(damage, 100.0); // 150 - 50
    }

    #[test]
    fn test_combat_engine_crit_and_dodge() {
        let mut attacker = StatResolver::new();
        attacker.register_source(StatId::from("ATK"), Box::new(ConstantSource(100.0)));
        attacker.register_source(StatId::from("CRIT_CHANCE"), Box::new(ConstantSource(0.20))); // 20%
        attacker.register_source(StatId::from("CRIT_MULT"), Box::new(ConstantSource(2.0)));

        let mut defender = StatResolver::new();
        defender.register_source(StatId::from("DEF"), Box::new(ConstantSource(0.0)));
        defender.register_source(StatId::from("DODGE_CHANCE"), Box::new(ConstantSource(0.10))); // 10%

        let ctx = StatContext::new();

        let json = r#"{
            "name": "Attack with Dodge and Crit",
            "expression": {
                "type": "Chance",
                "chance_expr": { "type": "Stat", "target": "defender", "stat": "DODGE_CHANCE" },
                "success_expr": { "type": "Constant", "value": 0.0 },
                "fail_expr": {
                    "type": "Chance",
                    "chance_expr": { "type": "Stat", "target": "attacker", "stat": "CRIT_CHANCE" },
                    "success_expr": {
                        "type": "Multiply",
                        "left": { "type": "Stat", "target": "attacker", "stat": "ATK" },
                        "right": { "type": "Stat", "target": "attacker", "stat": "CRIT_MULT" }
                    },
                    "fail_expr": { "type": "Stat", "target": "attacker", "stat": "ATK" }
                }
            }
        }"#;

        let formula: CombatFormula = serde_json::from_str(json).unwrap();

        // Scenario 1: Dodge (roll = 0.05 <= 0.10)
        let mut roll_seq = vec![0.05].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_dodge =
            CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng)
                .unwrap();
        assert_eq!(dmg_dodge, 0.0);

        // Scenario 2: Hit but no crit (roll 1 = 0.50 > 0.10 Dodge, roll 2 = 0.50 > 0.20 Crit)
        let mut roll_seq = vec![0.50, 0.50].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_normal =
            CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng)
                .unwrap();
        assert_eq!(dmg_normal, 100.0);

        // Scenario 3: Hit and crit (roll 1 = 0.50 > 0.10 Dodge, roll 2 = 0.15 <= 0.20 Crit)
        let mut roll_seq = vec![0.50, 0.15].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_crit =
            CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng)
                .unwrap();
        assert_eq!(dmg_crit, 200.0);
    }
}
