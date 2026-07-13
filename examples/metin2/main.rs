mod data;

#[cfg(test)]
mod tests;
use data::Metin2Data;
use zzstat::combat::{CombatEngine, CombatFormula};
use zzstat::context::StatContext;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;
use std::path::Path;

fn main() {
    println!("============================================================");
    println!("     METIN2 DAMAGE SIMULATOR (ZZSTAT IMPLEMENTATION)      ");
    println!("============================================================");

    let data_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples").join("metin2").join("data");
    let game_data = Metin2Data::load(&data_path).expect("Failed to load Metin2 JSON data");
    
    println!("✅ System Loaded: {} Weapons, {} Monsters.\n", game_data.weapons.len(), game_data.monsters.len());

    let weapon_vnum = 299; // Epée de bataille +9
    let weapon = game_data.weapons.get(&weapon_vnum).unwrap_or(&game_data.weapons.values().next().unwrap());
    let (min_att, max_att) = weapon.get_attack_values(9);
    
    // Warrior stats
    let strength = 90.0;
    let attack_other = 0.0;
    let level = 105.0;
    let main_attack = level * 2.0 + strength * 2.0; 
    let attack_factor = 1.0; 
    let avg_attack = (min_att + max_att) / 2.0;
    let raw_damage = main_attack + attack_factor * (2.0 * avg_attack + attack_other);

    println!("================== 👤 PLAYER DETAILS =======================");
    println!("Class: Warrior");
    println!("Level: {}", level);
    println!("STR: {}", strength);
    println!("Weapon ID: {} (Upgrade: +9)", weapon.id);
    println!("Weapon Attack Range: {} - {}", min_att, max_att);
    println!("Base Raw Damage: {:.2} (Before Enemy Defense & Bonuses)", raw_damage);
    println!("============================================================\n");

    let monster_ids = vec![
        101, // Chien errant
        691, // Chef orc
        1093 // Faucheuse
    ];

    let json_data = r#"{
        "name": "Metin2 Full Mathematical Formula",
        "expression": {
            "type": "Clamp",
            "min": 0.0,
            "expr": {
                "type": "Multiply",
                "left": {
                    "type": "Multiply",
                    "left": {
                        "type": "Multiply",
                        "left": {
                            "type": "Multiply",
                            "left": {
                                "type": "Chance",
                                "chance_expr": { "type": "Stat", "target": "attacker", "stat": "PIERCING_CHANCE" },
                                "success_expr": {
                                    "type": "Chance",
                                    "chance_expr": { "type": "Stat", "target": "attacker", "stat": "CRIT_CHANCE" },
                                    "success_expr": {
                                        "type": "Add",
                                        "left": {
                                            "type": "Add",
                                            "left": { "type": "Stat", "target": "attacker", "stat": "BASE_AFTER_DEFENSE" },
                                            "right": { "type": "Stat", "target": "attacker", "stat": "BASE_BEFORE_DEFENSE" }
                                        },
                                        "right": { "type": "Stat", "target": "defender", "stat": "DEFENSE" }
                                    },
                                    "fail_expr": {
                                        "type": "Add",
                                        "left": { "type": "Stat", "target": "attacker", "stat": "BASE_AFTER_DEFENSE" },
                                        "right": { "type": "Stat", "target": "defender", "stat": "DEFENSE" }
                                    }
                                },
                                "fail_expr": {
                                    "type": "Chance",
                                    "chance_expr": { "type": "Stat", "target": "attacker", "stat": "CRIT_CHANCE" },
                                    "success_expr": {
                                        "type": "Add",
                                        "left": { "type": "Stat", "target": "attacker", "stat": "BASE_AFTER_DEFENSE" },
                                        "right": { "type": "Stat", "target": "attacker", "stat": "BASE_BEFORE_DEFENSE" }
                                    },
                                    "fail_expr": {
                                        "type": "Stat", "target": "attacker", "stat": "BASE_AFTER_DEFENSE"
                                    }
                                }
                            },
                            "right": {
                                "type": "Add",
                                "left": { "type": "Constant", "value": 1.0 },
                                "right": { "type": "Stat", "target": "attacker", "stat": "WEAPON_BONUS" }
                            }
                        },
                        "right": {
                            "type": "Subtract",
                            "left": { "type": "Constant", "value": 1.0 },
                            "right": { "type": "Stat", "target": "defender", "stat": "AVERAGE_RESIST" }
                        }
                    },
                    "right": {
                        "type": "Add",
                        "left": { "type": "Constant", "value": 1.0 },
                        "right": { "type": "Stat", "target": "attacker", "stat": "SKILL_BONUS" }
                    }
                },
                "right": {
                    "type": "Subtract",
                    "left": { "type": "Constant", "value": 1.0 },
                    "right": { "type": "Stat", "target": "defender", "stat": "SKILL_RESIST" }
                }
            }
        }
    }"#;
    let formula: CombatFormula = serde_json::from_str(json_data).expect("JSON Parse Error");

    let ctx = StatContext::new();

    for m_id in monster_ids {
        let monster = game_data.monsters.get(&m_id).unwrap();
        println!("⚔️  FIGHT COMMENCING!");
        println!("================== 👾 TARGET DETAILS =======================");
        println!("Name: {}", monster.name);
        println!("Level: {}", monster.level());
        println!("Defense: {}", monster.defense());
        
        let grade = monster.data.get(0).and_then(|v| v.as_i64()).unwrap_or(0);
        println!("Grade: {}", grade);
        println!("============================================================");

        let mut attacker = StatResolver::new();
        let mut defender = StatResolver::new();

        // Let's add some varied bonuses based on the monster
        let mut race_bonus = 0.0;
        let mut monster_bonus = 0.0;
        let mut crit_chance = 0.20;
        let mut piercing_chance = 0.15;
        
        println!("--- Active Attack Bonuses ---");
        if monster.name == "Chef orc" {
            println!("(+) 50% Strong against Orcs");
            println!("(+) 40% Critical Chance");
            println!("(+) 15% Piercing Chance");
            race_bonus = 0.50; 
            crit_chance = 0.40;
        } else if monster.name == "Faucheuse" {
            println!("(+) 20% Strong against Undead");
            println!("(+) 30% Strong against Monsters");
            println!("(+) 20% Critical Chance");
            println!("(+) 50% Piercing Chance");
            race_bonus = 0.20;
            monster_bonus = 0.30;
            piercing_chance = 0.50;
        } else {
            println!("(+) 20% Critical Chance");
            println!("(+) 15% Piercing Chance");
        }

        // Apply primary bonuses mathematically 
        let base_before_defense = raw_damage
            * (1.0 + race_bonus) // raceBonus
            * (1.0 + monster_bonus); // monsterBonus

        let base_after_defense = f64::max(0.0, base_before_defense - monster.defense());

        attacker.register_source(StatId::from("BASE_BEFORE_DEFENSE"), Box::new(ConstantSource(base_before_defense)));
        attacker.register_source(StatId::from("BASE_AFTER_DEFENSE"), Box::new(ConstantSource(base_after_defense)));
        attacker.register_source(StatId::from("WEAPON_BONUS"), Box::new(ConstantSource(0.0)));
        attacker.register_source(StatId::from("SKILL_BONUS"), Box::new(ConstantSource(0.0)));
        attacker.register_source(StatId::from("CRIT_CHANCE"), Box::new(ConstantSource(crit_chance)));
        attacker.register_source(StatId::from("PIERCING_CHANCE"), Box::new(ConstantSource(piercing_chance)));

        defender.register_source(StatId::from("DEFENSE"), Box::new(ConstantSource(monster.defense())));
        defender.register_source(StatId::from("AVERAGE_RESIST"), Box::new(ConstantSource(0.0)));
        defender.register_source(StatId::from("SKILL_RESIST"), Box::new(ConstantSource(0.0)));

        // Run AST simulations
        let mut rolls_normal = vec![0.90, 0.90].into_iter(); // Both fail
        let mut rng_normal = || rolls_normal.next().unwrap_or(0.0);
        let dmg_normal = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_normal).unwrap();
        println!("   > [Normal Hit]   : {:.2} Damage", dmg_normal);

        let mut rolls_crit = vec![0.90, 0.05].into_iter(); // Pierce fail, Crit success
        let mut rng_crit = || rolls_crit.next().unwrap_or(0.0);
        let dmg_crit = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_crit).unwrap();
        println!("   > [Critical Hit] : {:.2} Damage", dmg_crit);
        
        let mut rolls_pierce = vec![0.05, 0.90].into_iter(); // Pierce success, Crit fail
        let mut rng_pierce = || rolls_pierce.next().unwrap_or(0.0);
        let dmg_pierce = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_pierce).unwrap();
        println!("   > [Piercing Hit] : {:.2} Damage", dmg_pierce);

        let mut rolls_crit_pierce = vec![0.05, 0.05].into_iter(); // Both success
        let mut rng_crit_pierce = || rolls_crit_pierce.next().unwrap_or(0.0);
        let dmg_crit_pierce = CombatEngine::evaluate(&formula, &mut attacker, &ctx, &mut defender, &ctx, &mut rng_crit_pierce).unwrap();
        println!("   > [Crit+Pierce]  : {:.2} Damage\n", dmg_crit_pierce);
    }
}
