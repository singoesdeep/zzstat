use std::fs;
use zzstat::context::StatContext;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;
use zzstat::transform::{AdditiveTransform, ScalingTransform, TransformPhase};
use zzstat::StatResolver;

fn main() {
    let mut resolver = StatResolver::new();

    // Stats
    resolver.register_source(StatId::from("LEVEL"), Box::new(ConstantSource(105.0)));
    resolver.register_source(StatId::from("STR"), Box::new(ConstantSource(90.0)));
    
    // Attack calculation (Level * 2 + Str * 2)
    resolver.register_transform_in_phase(
        StatId::from("MAIN_ATTACK"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("LEVEL"), 2.0)),
    );
    resolver.register_transform_in_phase(
        StatId::from("MAIN_ATTACK"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("STR"), 2.0)),
    );

    // Weapon
    resolver.register_source(StatId::from("WEAPON_MIN"), Box::new(ConstantSource(159.0)));
    resolver.register_source(StatId::from("WEAPON_MAX"), Box::new(ConstantSource(181.0)));
    resolver.register_transform_in_phase(
        StatId::from("WEAPON_AVG"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("WEAPON_MIN"), 0.5)),
    );
    resolver.register_transform_in_phase(
        StatId::from("WEAPON_AVG"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("WEAPON_MAX"), 0.5)),
    );

    // Base Raw Damage
    resolver.register_transform_in_phase(
        StatId::from("RAW_DAMAGE"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("MAIN_ATTACK"), 1.0)),
    );
    resolver.register_transform_in_phase(
        StatId::from("RAW_DAMAGE"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("WEAPON_AVG"), 2.0)),
    );

    // Defender
    resolver.register_source(StatId::from("MONSTER_DEFENSE"), Box::new(ConstantSource(146.0)));

    // Damage after defense (Simplified for graph)
    resolver.register_transform_in_phase(
        StatId::from("BASE_DAMAGE"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("RAW_DAMAGE"), 1.0)),
    );
    resolver.register_transform_in_phase(
        StatId::from("BASE_DAMAGE"),
        TransformPhase::Additive,
        Box::new(ScalingTransform::new(StatId::from("MONSTER_DEFENSE"), -1.0)),
    );

    let mermaid_code = resolver.export_mermaid();
    println!("{}", mermaid_code);
}
