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

/// VM tarafından işletilecek düz komut kodları (Bytecode).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Opcode {
    /// Sabit bir sayıyı yığına ekler.
    PushConstant(f64),
    /// Bir stat değerini okur ve yığına ekler.
    PushStat { target: CombatTarget, stat: StatId },
    /// Yığındaki son iki elemanı çeker, toplar ve sonucu yığına ekler.
    Add,
    /// Yığındaki son iki elemanı çeker, çıkarır (a - b) ve sonucu yığına ekler.
    Subtract,
    /// Yığındaki son iki elemanı çeker, çarpar ve sonucu yığına ekler.
    Multiply,
    /// Yığındaki son elemanı sınırlar.
    Clamp { min: Option<f64>, max: Option<f64> },
    /// Yığındaki son elemanı (ihtimal) çeker, RNG çalıştırır,
    /// eğer şans başarısız olursa belirtilen adresteki komuta atlar.
    ChanceJump { fail_idx: usize },
    /// Belirtilen adresteki komuta doğrudan atlar.
    Jump { target_idx: usize },
}

/// İsimlendirilmiş savaş formülü.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CombatFormula {
    pub name: String,
    pub expression: CombatExpression,
}

/// Önceden bytecode formatına derlenmiş savaş formülü.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CompiledFormula {
    pub name: String,
    pub bytecode: Vec<Opcode>,
}

impl CombatFormula {
    /// Formülü VM bytecode formatına derler.
    pub fn compile(&self) -> CompiledFormula {
        let mut bytecode = Vec::new();
        self.expression.compile_into(&mut bytecode);
        CompiledFormula {
            name: self.name.clone(),
            bytecode,
        }
    }
}

impl CombatExpression {
    /// İfadeyi bytecode (düz Opcode listesi) formatına derler.
    pub fn compile(&self) -> Vec<Opcode> {
        let mut bytecode = Vec::new();
        self.compile_into(&mut bytecode);
        bytecode
    }

    fn compile_into(&self, bytecode: &mut Vec<Opcode>) {
        match self {
            CombatExpression::Constant { value } => {
                bytecode.push(Opcode::PushConstant(*value));
            }
            CombatExpression::Stat { target, stat } => {
                let stat_id = StatId::from(stat.as_str());
                bytecode.push(Opcode::PushStat {
                    target: target.clone(),
                    stat: stat_id,
                });
            }
            CombatExpression::Add { left, right } => {
                left.compile_into(bytecode);
                right.compile_into(bytecode);
                bytecode.push(Opcode::Add);
            }
            CombatExpression::Subtract { left, right } => {
                left.compile_into(bytecode);
                right.compile_into(bytecode);
                bytecode.push(Opcode::Subtract);
            }
            CombatExpression::Multiply { left, right } => {
                left.compile_into(bytecode);
                right.compile_into(bytecode);
                bytecode.push(Opcode::Multiply);
            }
            CombatExpression::Clamp { min, max, expr } => {
                expr.compile_into(bytecode);
                bytecode.push(Opcode::Clamp {
                    min: *min,
                    max: *max,
                });
            }
            CombatExpression::Chance {
                chance_expr,
                success_expr,
                fail_expr,
            } => {
                // 1. Compile chance_expr
                chance_expr.compile_into(bytecode);

                // 2. ChanceJump placeholder ekle
                let chance_jump_idx = bytecode.len();
                bytecode.push(Opcode::ChanceJump { fail_idx: 0 });

                // 3. Compile success_expr
                success_expr.compile_into(bytecode);

                // 4. Jump placeholder ekle
                let jump_idx = bytecode.len();
                bytecode.push(Opcode::Jump { target_idx: 0 });

                // 5. ChanceJump hedef adresi (fail_expr'in başladığı yer)
                let fail_idx = bytecode.len();
                bytecode[chance_jump_idx] = Opcode::ChanceJump { fail_idx };

                // 6. Compile fail_expr
                fail_expr.compile_into(bytecode);

                // 7. Jump hedef adresi (fail_expr'in bittiği yer/program sonu)
                let end_idx = bytecode.len();
                bytecode[jump_idx] = Opcode::Jump {
                    target_idx: end_idx,
                };
            }
        }
    }
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

    /// Derlenmiş (bytecode) savaş formülünü sanal makine (VM) üzerinde hesaplar.
    pub fn evaluate_compiled<R>(
        formula: &CompiledFormula,
        attacker_resolver: &mut StatResolver,
        attacker_ctx: &StatContext,
        defender_resolver: &mut StatResolver,
        defender_ctx: &StatContext,
        rng: &mut R,
    ) -> Result<f64, StatError>
    where
        R: FnMut() -> f64,
    {
        Self::execute_vm(
            &formula.bytecode,
            attacker_resolver,
            attacker_ctx,
            defender_resolver,
            defender_ctx,
            rng,
        )
    }

    fn execute_vm<R>(
        bytecode: &[Opcode],
        attacker_resolver: &mut StatResolver,
        attacker_ctx: &StatContext,
        defender_resolver: &mut StatResolver,
        defender_ctx: &StatContext,
        rng: &mut R,
    ) -> Result<f64, StatError>
    where
        R: FnMut() -> f64,
    {
        let mut stack = Vec::with_capacity(16);
        let mut pc = 0;

        while pc < bytecode.len() {
            let op = &bytecode[pc];
            match op {
                Opcode::PushConstant(val) => {
                    stack.push(*val);
                    pc += 1;
                }
                Opcode::PushStat { target, stat } => {
                    let val = match target {
                        CombatTarget::Attacker => attacker_resolver.resolve(stat, attacker_ctx)?,
                        CombatTarget::Defender => defender_resolver.resolve(stat, defender_ctx)?,
                    };
                    stack.push(val.value.to_f64());
                    pc += 1;
                }
                Opcode::Add => {
                    let b = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Add".to_string())
                    })?;
                    let a = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Add".to_string())
                    })?;
                    stack.push(a + b);
                    pc += 1;
                }
                Opcode::Subtract => {
                    let b = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Subtract".to_string())
                    })?;
                    let a = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Subtract".to_string())
                    })?;
                    stack.push(a - b);
                    pc += 1;
                }
                Opcode::Multiply => {
                    let b = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Multiply".to_string())
                    })?;
                    let a = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Multiply".to_string())
                    })?;
                    stack.push(a * b);
                    pc += 1;
                }
                Opcode::Clamp { min, max } => {
                    let mut val = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during Clamp".to_string())
                    })?;
                    if let Some(m) = min {
                        val = val.max(*m);
                    }
                    if let Some(m) = max {
                        val = val.min(*m);
                    }
                    stack.push(val);
                    pc += 1;
                }
                Opcode::ChanceJump { fail_idx } => {
                    let chance = stack.pop().ok_or_else(|| {
                        StatError::VmError("Stack underflow during ChanceJump".to_string())
                    })?;
                    let roll = rng();
                    if roll <= chance {
                        pc += 1;
                    } else {
                        pc = *fail_idx;
                    }
                }
                Opcode::Jump { target_idx } => {
                    pc = *target_idx;
                }
            }
        }

        stack
            .pop()
            .ok_or_else(|| StatError::VmError("Empty stack at VM termination".to_string()))
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

    #[test]
    fn test_combat_engine_compiled_basic_damage() {
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

        let compiled = formula.compile();
        let damage = CombatEngine::evaluate_compiled(
            &compiled,
            &mut attacker,
            &ctx,
            &mut defender,
            &ctx,
            &mut || 0.0,
        )
        .unwrap();

        assert_eq!(damage, 100.0);
    }

    #[test]
    fn test_combat_engine_compiled_crit_and_dodge() {
        let mut attacker = StatResolver::new();
        attacker.register_source(StatId::from("ATK"), Box::new(ConstantSource(100.0)));
        attacker.register_source(StatId::from("CRIT_CHANCE"), Box::new(ConstantSource(0.20)));
        attacker.register_source(StatId::from("CRIT_MULT"), Box::new(ConstantSource(2.0)));

        let mut defender = StatResolver::new();
        defender.register_source(StatId::from("DEF"), Box::new(ConstantSource(0.0)));
        defender.register_source(StatId::from("DODGE_CHANCE"), Box::new(ConstantSource(0.10)));

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
        let compiled = formula.compile();

        // Scenario 1: Dodge (roll = 0.05 <= 0.10)
        let mut roll_seq = vec![0.05].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_dodge = CombatEngine::evaluate_compiled(
            &compiled,
            &mut attacker,
            &ctx,
            &mut defender,
            &ctx,
            &mut rng,
        )
        .unwrap();
        assert_eq!(dmg_dodge, 0.0);

        // Scenario 2: Hit but no crit
        let mut roll_seq = vec![0.50, 0.50].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_normal = CombatEngine::evaluate_compiled(
            &compiled,
            &mut attacker,
            &ctx,
            &mut defender,
            &ctx,
            &mut rng,
        )
        .unwrap();
        assert_eq!(dmg_normal, 100.0);

        // Scenario 3: Hit and crit
        let mut roll_seq = vec![0.50, 0.15].into_iter();
        let mut rng = || roll_seq.next().unwrap();
        let dmg_crit = CombatEngine::evaluate_compiled(
            &compiled,
            &mut attacker,
            &ctx,
            &mut defender,
            &ctx,
            &mut rng,
        )
        .unwrap();
        assert_eq!(dmg_crit, 200.0);
    }
}
