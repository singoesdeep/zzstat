//! Example 01: JSON Template Loader & Dynamic Conditions
//!
//! Demonstrates how `zzstat` loads stat structures completely from JSON
//! without hardcoding logic, and how it uses the `Condition` engine to
//! evaluate dynamic combat stances (e.g., DEFENSIVE).

use zzstat::context::StatContext;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;
use zzstat::template::StatTemplate;
use zzstat::StatNumeric;

fn main() {
    // 1. Define the logic in JSON (Usually loaded from a file or database)
    // This JSON says:
    // "Base DEF is 50. If the player is in DEFENSIVE stance, add 30 DEF. Otherwise, add 0."
    let json_data = r#"{
        "name": "Knight Class Core Stats",
        "transforms": [
            {
                "stat": "DEF",
                "phase": "additive",
                "rule": "additive",
                "type": "Additive",
                "value": 50.0
            },
            {
                "stat": "DEF",
                "phase": "additive",
                "rule": "additive",
                "type": "Conditional",
                "condition": { "operator": "Equals", "key": "STANCE", "value": "DEFENSIVE" },
                "transform": { "type": "Additive", "value": 30.0 }
            }
        ]
    }"#;

    // 2. Deserialize the JSON into our Rust models
    let template: StatTemplate = serde_json::from_str(json_data).unwrap();

    // 3. Build a StatResolver from the template
    let mut resolver = template.build_resolver().unwrap();

    // We must register a base source for "DEF" even if it's 0 to allow transforms
    let def_id = StatId::from("DEF");
    resolver.register_source(def_id.clone(), Box::new(ConstantSource(0.0)));

    println!("--- JSON Template Loaded ---");
    println!("Template Name: {}\n", template.name);

    // 4. Test Scenario A: Normal Stance
    let mut ctx_normal = StatContext::new();
    ctx_normal.set("STANCE", "NORMAL");

    let def_normal = resolver.resolve(&def_id, &ctx_normal).unwrap();
    println!("DEF in NORMAL stance: {}", def_normal.value.to_f64()); // Expect: 50.0

    // 5. Test Scenario B: Defensive Stance
    let mut ctx_defensive = StatContext::new();
    ctx_defensive.set("STANCE", "DEFENSIVE");

    resolver.invalidate_all();
    let def_defensive = resolver.resolve(&def_id, &ctx_defensive).unwrap();
    println!("DEF in DEFENSIVE stance: {}", def_defensive.value.to_f64()); // Expect: 80.0
}
