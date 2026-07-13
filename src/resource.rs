//! Resource pool management and time-based effects.
//!
//! Provides a `ResourcePool` to track stateful values like Current HP or MP,
//! along with Damage over Time (DoT) and Heal over Time (HoT) effects.

use crate::context::StatContext;
use crate::numeric::StatNumeric;
use crate::resolver::StatResolver;
use crate::stat_id::StatId;
use serde::{Deserialize, Serialize};

/// Condition for a threshold trigger.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TriggerCondition {
    /// Triggers when the resource falls strictly below this value.
    Below(f64),
    /// Triggers when the resource rises strictly above this value.
    Above(f64),
    /// Triggers when the resource is exactly 0 (or lower).
    Empty,
    /// Triggers when the resource is exactly at its max capacity (or higher).
    Full,
}

/// A trigger that emits an event when a condition is met.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ThresholdTrigger {
    pub condition: TriggerCondition,
    pub event_name: String,
}

/// A time-based effect applied to a resource.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TimeEffect {
    /// A unique identifier for this effect (e.g., "Poison", "Regen").
    pub name: String,
    /// The amount to change the resource per tick.
    /// Negative values reduce the resource (DoT), positive increase it (HoT).
    pub amount_per_tick: f64,
    /// How many ticks this effect will last.
    pub ticks_remaining: u32,
}

/// Emitted when a threshold trigger condition is met.
#[derive(Debug, Clone, PartialEq)]
pub struct ResourceEvent {
    pub event_name: String,
}

/// A stateful resource pool (e.g., Current HP) bounded by a maximum stat.
#[derive(Debug, Clone)]
pub struct ResourcePool {
    /// The stat ID that defines the maximum capacity of this pool (e.g., "MAX_HP").
    pub max_stat_id: StatId,
    /// The current value of the resource.
    pub current_value: f64,
    /// Active time-based effects.
    pub active_effects: Vec<TimeEffect>,
    /// Triggers for this resource.
    pub triggers: Vec<ThresholdTrigger>,
}

impl ResourcePool {
    /// Creates a new ResourcePool starting at full capacity based on the current resolver state.
    pub fn new(max_stat_id: StatId, resolver: &mut StatResolver, context: &StatContext) -> Self {
        let max_val = resolver
            .resolve(&max_stat_id, context)
            .map(|r| r.value.to_f64())
            .unwrap_or(0.0);

        Self {
            max_stat_id,
            current_value: max_val,
            active_effects: Vec::new(),
            triggers: Vec::new(),
        }
    }

    /// Adds a time effect (DoT or HoT) to the pool.
    pub fn add_effect(&mut self, effect: TimeEffect) {
        self.active_effects.push(effect);
    }

    /// Adds a threshold trigger to the pool.
    pub fn add_trigger(&mut self, trigger: ThresholdTrigger) {
        self.triggers.push(trigger);
    }

    /// Explicitly applies direct damage to the resource.
    /// Returns any events triggered by the change.
    pub fn apply_damage(
        &mut self,
        amount: f64,
        resolver: &mut StatResolver,
        context: &StatContext,
    ) -> Vec<ResourceEvent> {
        // Damage reduces the value, so delta is negative
        self.change_value(-amount, resolver, context)
    }

    /// Explicitly applies direct healing to the resource.
    /// Returns any events triggered by the change.
    pub fn apply_heal(
        &mut self,
        amount: f64,
        resolver: &mut StatResolver,
        context: &StatContext,
    ) -> Vec<ResourceEvent> {
        self.change_value(amount, resolver, context)
    }

    /// Advances the pool by one tick, applying active effects.
    /// Returns any events triggered by the state change.
    pub fn tick(
        &mut self,
        resolver: &mut StatResolver,
        context: &StatContext,
    ) -> Vec<ResourceEvent> {
        let mut total_change = 0.0;

        self.active_effects.retain_mut(|effect| {
            if effect.ticks_remaining > 0 {
                total_change += effect.amount_per_tick;
                effect.ticks_remaining -= 1;
                effect.ticks_remaining > 0
            } else {
                false
            }
        });

        if total_change != 0.0 {
            self.change_value(total_change, resolver, context)
        } else {
            // Even if no time effects changed the value, check triggers
            // in case max capacity changed or existing state meets conditions.
            self.check_triggers(resolver, context)
        }
    }

    /// Internal function to apply a delta and check triggers.
    fn change_value(
        &mut self,
        delta: f64,
        resolver: &mut StatResolver,
        context: &StatContext,
    ) -> Vec<ResourceEvent> {
        let max_val = resolver
            .resolve(&self.max_stat_id, context)
            .map(|r| r.value.to_f64())
            .unwrap_or(0.0);

        self.current_value = (self.current_value + delta).clamp(0.0, max_val);
        self.check_triggers(resolver, context)
    }

    /// Evaluates all triggers against the current state.
    fn check_triggers(
        &self,
        resolver: &mut StatResolver,
        context: &StatContext,
    ) -> Vec<ResourceEvent> {
        let mut events = Vec::new();
        let max_val = resolver
            .resolve(&self.max_stat_id, context)
            .map(|r| r.value.to_f64())
            .unwrap_or(0.0);

        for trigger in &self.triggers {
            let is_met = match trigger.condition {
                TriggerCondition::Below(threshold) => self.current_value < threshold,
                TriggerCondition::Above(threshold) => self.current_value > threshold,
                TriggerCondition::Empty => self.current_value <= 0.0,
                TriggerCondition::Full => self.current_value >= max_val,
            };

            if is_met {
                events.push(ResourceEvent {
                    event_name: trigger.event_name.clone(),
                });
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ConstantSource;

    #[test]
    fn test_resource_pool_tick_and_triggers() {
        let mut resolver = StatResolver::new();
        let context = StatContext::new();
        let max_hp_id = StatId::from("MAX_HP");

        // Set MAX_HP to 100
        resolver.register_source(max_hp_id.clone(), Box::new(ConstantSource(100.0)));

        let mut pool = ResourcePool::new(max_hp_id, &mut resolver, &context);
        assert_eq!(pool.current_value, 100.0);

        // Add a trigger for Death
        pool.add_trigger(ThresholdTrigger {
            condition: TriggerCondition::Empty,
            event_name: "DEATH".to_string(),
        });

        // Add a Poison DoT: -10 HP per tick for 3 ticks
        pool.add_effect(TimeEffect {
            name: "Poison".to_string(),
            amount_per_tick: -10.0,
            ticks_remaining: 3,
        });

        // Tick 1
        let events = pool.tick(&mut resolver, &context);
        assert_eq!(pool.current_value, 90.0);
        assert!(events.is_empty());

        // Tick 2
        pool.tick(&mut resolver, &context);
        assert_eq!(pool.current_value, 80.0);

        // Tick 3
        pool.tick(&mut resolver, &context);
        assert_eq!(pool.current_value, 70.0);

        // Tick 4 (Effect should be gone)
        pool.tick(&mut resolver, &context);
        assert_eq!(pool.current_value, 70.0); // Remains 70

        // Apply fatal damage
        let events = pool.apply_damage(100.0, &mut resolver, &context);
        assert_eq!(pool.current_value, 0.0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "DEATH");
    }

    #[test]
    fn test_resource_pool_healing_and_full() {
        let mut resolver = StatResolver::new();
        let context = StatContext::new();
        let max_hp_id = StatId::from("MAX_HP");

        resolver.register_source(max_hp_id.clone(), Box::new(ConstantSource(100.0)));

        let mut pool = ResourcePool::new(max_hp_id, &mut resolver, &context);
        pool.apply_damage(50.0, &mut resolver, &context); // Drop to 50
        assert_eq!(pool.current_value, 50.0);

        pool.add_trigger(ThresholdTrigger {
            condition: TriggerCondition::Full,
            event_name: "FULLY_HEALED".to_string(),
        });

        // Regen HoT: +20 HP per tick for 3 ticks
        pool.add_effect(TimeEffect {
            name: "Regen".to_string(),
            amount_per_tick: 20.0,
            ticks_remaining: 3,
        });

        // Tick 1 (70)
        pool.tick(&mut resolver, &context);
        // Tick 2 (90)
        pool.tick(&mut resolver, &context);
        // Tick 3 (100 - capped)
        let events = pool.tick(&mut resolver, &context);
        assert_eq!(pool.current_value, 100.0);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "FULLY_HEALED");
    }
}
