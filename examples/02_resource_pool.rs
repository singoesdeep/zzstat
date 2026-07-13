//! Example 02: Resource Pool (HP), Time Effects (DoT), and Triggers
//!
//! Demonstrates the stateful side of `zzstat`. How to track an entity's 
//! Current HP, apply Poison (Damage over Time), and handle death triggers.

use zzstat::context::StatContext;
use zzstat::resolver::StatResolver;
use zzstat::stat_id::StatId;
use zzstat::source::ConstantSource;
use zzstat::resource::{ResourcePool, TimeEffect, ThresholdTrigger, TriggerCondition};

fn main() {
    let mut resolver = StatResolver::new();
    let ctx = StatContext::new();

    // 1. Define MAX_HP in the resolver
    let max_hp_id = StatId::from("MAX_HP");
    resolver.register_source(max_hp_id.clone(), Box::new(ConstantSource(100.0)));

    // 2. Initialize a stateful Resource Pool linked to MAX_HP
    let mut hp_pool = ResourcePool::new(max_hp_id, &mut resolver, &ctx);
    println!("Initial HP: {} / {}", hp_pool.current_value, 100.0);

    // 3. Add a Death Trigger (Triggers when HP is 0 or less)
    hp_pool.add_trigger(ThresholdTrigger {
        condition: TriggerCondition::Empty,
        event_name: "DEATH".to_string(),
    });

    // 4. Add a "Poison" effect: -20 HP per tick, lasts for 3 ticks
    hp_pool.add_effect(TimeEffect {
        name: "Poison".to_string(),
        amount_per_tick: -20.0,
        ticks_remaining: 3,
    });
    println!("Poison applied! (-20 HP per tick for 3 ticks)");

    // 5. Simulate Game Loop (Ticks)
    for tick in 1..=4 {
        println!("--- Tick {} ---", tick);
        let events = hp_pool.tick(&mut resolver, &ctx);
        
        println!("Current HP: {}", hp_pool.current_value);

        for event in events {
            println!(">>> TRIGGER FIRED: {} <<<", event.event_name);
        }
    }

    // 6. Direct Damage (Executing a big hit)
    println!("--- Monster hits for 50 damage! ---");
    let events = hp_pool.apply_damage(50.0, &mut resolver, &ctx);
    println!("Current HP: {}", hp_pool.current_value);
    
    for event in events {
        println!(">>> TRIGGER FIRED: {} <<<", event.event_name);
    }
}
