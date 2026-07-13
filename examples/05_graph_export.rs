use std::fs;
use zzstat::context::StatContext;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;
use zzstat::transform::{AdditiveTransform, ScalingTransform, TransformPhase};
use zzstat::StatResolver;

fn main() {
    println!("Generating Map Visualization for ZZSTAT...");

    let mut resolver = StatResolver::new();

    // Base sources (e.g. from JSON Templates or default class stats)
    resolver.register_source(StatId::from("STR"), Box::new(ConstantSource(50.0)));
    resolver.register_source(StatId::from("DEX"), Box::new(ConstantSource(30.0)));
    resolver.register_source(StatId::from("BASE_ATK"), Box::new(ConstantSource(100.0)));

    // Active Bonuses (e.g. from an equipped item like Sword+9)
    resolver.register_source(StatId::from("BASE_ATK"), Box::new(ConstantSource(200.0)));
    resolver.register_source(StatId::from("STR"), Box::new(ConstantSource(10.0)));

    // Dependencies and Formulas (transforms)
    // ATK = BASE_ATK + STR * 2.0
    resolver.register_transform_in_phase(
        StatId::from("ATK"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("STR"), 2.0)),
    );
    resolver.register_transform_in_phase(
        StatId::from("ATK"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("BASE_ATK"), 1.0)),
    );

    // CRIT_CHANCE = 5.0 + DEX * 0.1
    resolver.register_source(StatId::from("CRIT_CHANCE"), Box::new(ConstantSource(5.0)));
    resolver.register_transform_in_phase(
        StatId::from("CRIT_CHANCE"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("DEX"), 0.1)),
    );

    // DPS = ATK * (1.0 + CRIT_CHANCE * 0.5 / 100)
    // We add a simplified additive transform for the sake of the graph
    resolver.register_transform_in_phase(
        StatId::from("DPS"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("ATK"), 1.0)),
    );

    let mermaid_code = resolver.export_mermaid();

    let html_content = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>ZZSTAT - Stat Dependency Map</title>
    <script src="https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.min.js"></script>
    <script>
        mermaid.initialize({{ startOnLoad: true, theme: 'dark' }});
    </script>
    <style>
        body {{ background-color: #1a1a1a; color: white; font-family: sans-serif; display: flex; flex-direction: column; align-items: center; padding: 20px; }}
        .mermaid {{ background: #2a2a2a; padding: 20px; border-radius: 10px; box-shadow: 0 4px 10px rgba(0,0,0,0.5); }}
    </style>
</head>
<body>
    <h1>ZZSTAT Dependency Map</h1>
    <p>This map shows how active items/bonuses flow into base stats, and how stats feed into each other via formulas.</p>
    <div class="mermaid">
{}
    </div>
</body>
</html>"#,
        mermaid_code
    );

    let path = "graph_export.html";
    fs::write(path, html_content).expect("Failed to write HTML file");

    println!("✅ Visualization generated successfully!");
    println!(
        "   -> Open '{}' in your browser to view the interactive map.",
        path
    );
    println!("   -> You can also paste the Mermaid code into https://mermaid.live");
}
