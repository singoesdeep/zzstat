use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use std::collections::HashMap;
use std::time::Duration;

mod data;
use data::*;

use zzstat::combat::{CombatEngine, CombatFormula};
use zzstat::context::StatContext;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    #[default]
    Loading,
    Running,
}

#[derive(Resource)]
struct GameDataAssets {
    weapons: Handle<JsonAsset>,
    monsters: Handle<JsonAsset>,
    constants: Handle<JsonAsset>,
}

#[derive(Resource)]
struct GameData(Metin2Data);

#[derive(Resource)]
struct CombatContext {
    formula: CombatFormula,
    ctx: StatContext,
}

// Components
#[derive(Component)]
struct Player {
    weapon_vnum: u32,
    raw_damage: f64,
}

#[derive(Component)]
struct Monster {
    id: u32,
    name: String,
    defense: f64,
    race_bonus: f64,
    monster_bonus: f64,
    crit_chance: f64,
    piercing_chance: f64,
}

#[derive(Component)]
struct ShowcaseSequence {
    step: usize,
}

// Events
#[derive(Event)]
struct AttackEvent {
    attacker: Entity,
    defender: Entity,
    step: usize,
}

#[derive(Event)]
struct DamageEvent {
    target_name: String,
    amount: f64,
    hit_type: &'static str,
}

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                1.0 / 60.0,
            ))),
        )
        .add_plugins(bevy::asset::AssetPlugin {
            file_path: "examples/metin2/data".to_string(),
            ..default()
        })
        .add_plugins(bevy::state::app::StatesPlugin)
        .init_state::<AppState>()
        .init_asset::<JsonAsset>()
        .init_asset_loader::<JsonLoader>()
        .add_event::<AttackEvent>()
        .add_event::<DamageEvent>()
        .add_systems(OnEnter(AppState::Loading), load_assets)
        .add_systems(
            Update,
            check_assets_loaded.run_if(in_state(AppState::Loading)),
        )
        .add_systems(OnEnter(AppState::Running), setup_game)
        .add_systems(
            Update,
            (trigger_combat, resolve_combat, log_damage)
                .chain()
                .run_if(in_state(AppState::Running)),
        )
        .run();
}

fn load_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    println!("Loading game data via Bevy AssetServer...");
    commands.insert_resource(GameDataAssets {
        weapons: asset_server.load("weapons.json"),
        monsters: asset_server.load("monsters.json"),
        constants: asset_server.load("constants.json"),
    });
}

fn check_assets_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    assets: Res<GameDataAssets>,
    json_assets: Res<Assets<JsonAsset>>,
    mut state: ResMut<NextState<AppState>>,
) {
    use bevy::asset::LoadState;
    let w_load_state = asset_server.load_state(&assets.weapons);
    let m_load_state = asset_server.load_state(&assets.monsters);
    let c_load_state = asset_server.load_state(&assets.constants);

    if w_load_state == LoadState::Loaded
        && m_load_state == LoadState::Loaded
        && c_load_state == LoadState::Loaded
    {
        let weapons_str = &json_assets.get(&assets.weapons).unwrap().0;
        let monsters_str = &json_assets.get(&assets.monsters).unwrap().0;
        let constants_str = &json_assets.get(&assets.constants).unwrap().0;

        let w_list: Vec<WeaponDef> = serde_json::from_str(weapons_str).unwrap();
        let m_list: Vec<MonsterDef> = serde_json::from_str(monsters_str).unwrap();
        let constants: ConstantsDef = serde_json::from_str(constants_str).unwrap();

        let mut weapons = HashMap::new();
        for w in w_list {
            weapons.insert(w.id, w);
        }

        let mut monsters = HashMap::new();
        for m in m_list {
            monsters.insert(m.id, m);
        }

        commands.insert_resource(GameData(Metin2Data {
            weapons,
            monsters,
            constants,
        }));

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
        commands.insert_resource(CombatContext {
            formula,
            ctx: StatContext::new(),
        });

        println!("Assets loaded successfully!");
        state.set(AppState::Running);
    } else if matches!(w_load_state, LoadState::Failed(_))
        || matches!(m_load_state, LoadState::Failed(_))
        || matches!(c_load_state, LoadState::Failed(_))
    {
        panic!("Failed to load assets.");
    }
}

fn setup_game(mut commands: Commands, game_data: Res<GameData>) {
    println!("============================================================");
    println!("     METIN2 DAMAGE SIMULATOR (BEVY ECS INTEGRATION)       ");
    println!("============================================================");

    let data = &game_data.0;
    println!(
        "✅ System Loaded: {} Weapons, {} Monsters.\n",
        data.weapons.len(),
        data.monsters.len()
    );

    let weapon_vnum = 299; // Epée de bataille +9
    let weapon = data
        .weapons
        .get(&weapon_vnum)
        .unwrap_or_else(|| data.weapons.values().next().unwrap());
    let (min_att, max_att) = weapon.get_attack_values(9);

    let strength = 90.0;
    let attack_other = 0.0;
    let level = 105.0;
    let main_attack = level * 2.0 + strength * 2.0;
    let avg_attack = (min_att + max_att) / 2.0;
    let raw_damage = main_attack + (2.0 * avg_attack + attack_other);

    println!("================== 👤 PLAYER DETAILS =======================");
    println!("Class: Warrior");
    println!("Level: {}", level);
    println!("STR: {}", strength);
    println!("Weapon ID: {} (Upgrade: +9)", weapon.id);
    println!("Weapon Attack Range: {} - {}", min_att, max_att);
    println!(
        "Base Raw Damage: {:.2} (Before Enemy Defense & Bonuses)",
        raw_damage
    );
    println!("============================================================\n");

    commands.spawn((Player {
        weapon_vnum,
        raw_damage,
    },));

    let monster_ids = vec![101, 691, 1093];

    for m_id in monster_ids {
        let monster_def = data.monsters.get(&m_id).unwrap();

        let mut race_bonus = 0.0;
        let mut monster_bonus = 0.0;
        let mut crit_chance = 0.20;
        let mut piercing_chance = 0.15;

        println!("--- Preparing Target: {} ---", monster_def.name);
        if monster_def.name == "Chef orc" {
            println!("(+) 50% Strong against Orcs");
            println!("(+) 40% Critical Chance");
            println!("(+) 15% Piercing Chance");
            race_bonus = 0.50;
            crit_chance = 0.40;
        } else if monster_def.name == "Faucheuse" {
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

        commands.spawn((
            Monster {
                id: monster_def.id,
                name: monster_def.name.clone(),
                defense: monster_def.defense(),
                race_bonus,
                monster_bonus,
                crit_chance,
                piercing_chance,
            },
            ShowcaseSequence { step: 0 },
        ));
    }
}

fn trigger_combat(
    time: Res<Time>,
    mut timer: Local<f32>,
    player_q: Query<Entity, With<Player>>,
    mut monster_q: Query<(Entity, &mut ShowcaseSequence), With<Monster>>,
    mut attack_ev: EventWriter<AttackEvent>,
) {
    *timer += time.delta_seconds();
    if *timer > 0.5 {
        // Trigger an attack every 0.5s for visual cadence
        *timer = 0.0;

        if let Ok(player_ent) = player_q.get_single() {
            // Find the first monster that still needs to be attacked
            for (monster_ent, mut seq) in monster_q.iter_mut() {
                if seq.step < 4 {
                    attack_ev.send(AttackEvent {
                        attacker: player_ent,
                        defender: monster_ent,
                        step: seq.step,
                    });
                    seq.step += 1;
                    break;
                }
            }
        }
    }
}

fn resolve_combat(
    mut attack_ev: EventReader<AttackEvent>,
    mut damage_ev: EventWriter<DamageEvent>,
    player_q: Query<&Player>,
    monster_q: Query<(&Monster, &ShowcaseSequence)>,
    combat_ctx: Res<CombatContext>,
    mut app_exit: EventWriter<bevy::app::AppExit>,
) {
    let mut all_done = true;
    for (_, seq) in monster_q.iter() {
        if seq.step < 4 {
            all_done = false;
            break;
        }
    }

    for ev in attack_ev.read() {
        if let Ok(player) = player_q.get(ev.attacker) {
            if let Ok((monster, _)) = monster_q.get(ev.defender) {
                if ev.step == 0 {
                    println!("\n⚔️  FIGHT COMMENCING!");
                    println!("================== 👾 TARGET DETAILS =======================");
                    println!("Name: {}", monster.name);
                    println!("Defense: {}", monster.defense);
                    println!("============================================================");
                }

                let hit_types = ["Normal Hit", "Critical Hit", "Piercing Hit", "Crit+Pierce"];
                let (crit_roll, pierce_roll) = match ev.step {
                    0 => (0.90, 0.90),
                    1 => (0.05, 0.90),
                    2 => (0.90, 0.05),
                    3 => (0.05, 0.05),
                    _ => unreachable!(),
                };

                let mut rolls = vec![pierce_roll, crit_roll].into_iter();
                let mut rng = || rolls.next().unwrap_or(0.0);

                let mut a_res = StatResolver::new();
                let mut d_res = StatResolver::new();

                a_res.register_source(StatId::from("WEAPON_BONUS"), Box::new(ConstantSource(0.0)));
                a_res.register_source(StatId::from("SKILL_BONUS"), Box::new(ConstantSource(0.0)));

                d_res.register_source(
                    StatId::from("DEFENSE"),
                    Box::new(ConstantSource(monster.defense)),
                );
                d_res.register_source(
                    StatId::from("AVERAGE_RESIST"),
                    Box::new(ConstantSource(0.0)),
                );
                d_res.register_source(StatId::from("SKILL_RESIST"), Box::new(ConstantSource(0.0)));

                let base_before_defense =
                    player.raw_damage * (1.0 + monster.race_bonus) * (1.0 + monster.monster_bonus);

                let base_after_defense = f64::max(0.0, base_before_defense - monster.defense);

                a_res.register_source(
                    StatId::from("BASE_BEFORE_DEFENSE"),
                    Box::new(ConstantSource(base_before_defense)),
                );
                a_res.register_source(
                    StatId::from("BASE_AFTER_DEFENSE"),
                    Box::new(ConstantSource(base_after_defense)),
                );
                a_res.register_source(
                    StatId::from("CRIT_CHANCE"),
                    Box::new(ConstantSource(monster.crit_chance)),
                );
                a_res.register_source(
                    StatId::from("PIERCING_CHANCE"),
                    Box::new(ConstantSource(monster.piercing_chance)),
                );

                let dmg = CombatEngine::evaluate(
                    &combat_ctx.formula,
                    &mut a_res,
                    &combat_ctx.ctx,
                    &mut d_res,
                    &combat_ctx.ctx,
                    &mut rng,
                )
                .unwrap();

                damage_ev.send(DamageEvent {
                    target_name: monster.name.clone(),
                    amount: dmg,
                    hit_type: hit_types[ev.step],
                });
            }
        }
    }

    if all_done && attack_ev.is_empty() {
        app_exit.send(bevy::app::AppExit::Success);
    }
}

fn log_damage(mut damage_ev: EventReader<DamageEvent>) {
    for ev in damage_ev.read() {
        println!(
            "   > [{}] : {:.2} Damage to {}",
            ev.hit_type, ev.amount, ev.target_name
        );
    }
}
