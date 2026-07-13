use crate::data::Metin2Data;
use zzstat::combat::{CombatEngine, CombatFormula};
use zzstat::context::StatContext;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;

#[test]
fn test_simple_damage_calculation() {
    let json_data = r#"{
        "name": "Metin2 Simplified Formula",
        "expression": {
            "type": "Clamp",
            "min": 0.0,
            "expr": {
                "type": "Subtract",
                "left": {
                    "type": "Stat",
                    "target": "attacker",
                    "stat": "BASE_DAMAGE"
                },
                "right": {
                    "type": "Stat",
                    "target": "defender",
                    "stat": "DEFENSE"
                }
            }
        }
    }"#;
    let formula: CombatFormula = serde_json::from_str(json_data).expect("JSON Parse Error");

    let mut attacker = StatResolver::new();
    let mut defender = StatResolver::new();

    attacker.register_source(StatId::from("BASE_DAMAGE"), Box::new(ConstantSource(500.0)));
    defender.register_source(StatId::from("DEFENSE"), Box::new(ConstantSource(200.0)));

    let ctx = StatContext::new();
    let mut rng = || 0.0;

    let dmg = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng).unwrap();
    assert_eq!(dmg, 300.0); // 500 - 200
}

#[test]
fn test_critical_hit_multiplier() {
    let json_data = r#"{
        "name": "Crit Test",
        "expression": {
            "type": "Chance",
            "chance_expr": { "type": "Stat", "target": "attacker", "stat": "CRIT_CHANCE" },
            "success_expr": {
                "type": "Multiply",
                "left": { "type": "Stat", "target": "attacker", "stat": "BASE_DAMAGE" },
                "right": { "type": "Constant", "value": 2.0 }
            },
            "fail_expr": { "type": "Stat", "target": "attacker", "stat": "BASE_DAMAGE" }
        }
    }"#;
    let formula: CombatFormula = serde_json::from_str(json_data).expect("JSON Parse Error");

    let mut attacker = StatResolver::new();
    let mut defender = StatResolver::new();

    attacker.register_source(StatId::from("BASE_DAMAGE"), Box::new(ConstantSource(100.0)));
    attacker.register_source(StatId::from("CRIT_CHANCE"), Box::new(ConstantSource(0.5)));

    let ctx = StatContext::new();
    
    // Simulate successful crit
    let mut rng_success = || 0.1; // 0.1 < 0.5 -> success
    let dmg_success = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_success).unwrap();
    assert_eq!(dmg_success, 200.0);
    
    // Simulate failed crit
    let mut rng_fail = || 0.9; // 0.9 > 0.5 -> fail
    let dmg_fail = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_fail).unwrap();
    assert_eq!(dmg_fail, 100.0);
}
