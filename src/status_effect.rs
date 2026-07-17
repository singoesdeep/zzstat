//! Status Effects (Buff/Debuff) module.
//!
//! Provides a `StatusEffectManager` to handle temporary effects that modify stats.
//! It uses `StatResolver::fork()` to apply bonuses efficiently without modifying
//! the base stats.

use crate::bonus::{apply_compiled_bonuses, compile_bonus, Bonus, CompiledBonus};
use crate::resolver::StatResolver;
use serde::{Deserialize, Serialize};

/// Defines how a status effect behaves when applied multiple times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StackBehavior {
    /// Resets the duration, but keeps the stack count the same.
    Refresh,
    /// Increases the stack count up to `max_stacks` and optionally resets duration.
    Accumulate { reset_duration: bool },
    /// Independent instances; each application runs its own timer.
    Independent,
}

/// A definition of a status effect (e.g., a buff or debuff).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusEffect {
    /// Unique identifier for this effect type (e.g., "BUFF_WARCRY").
    pub id: String,
    /// Display name.
    pub name: String,
    /// The bonuses this effect provides per stack.
    pub bonuses: Vec<Bonus>,
    /// Maximum number of stacks allowed.
    pub max_stacks: u32,
    /// How this effect stacks when reapplied.
    pub stack_behavior: StackBehavior,
}

/// An active instance of a status effect on an entity.
#[derive(Debug, Clone)]
pub struct ActiveStatusEffect {
    pub effect: StatusEffect,
    pub current_stacks: u32,
    pub duration_ticks: Option<u32>, // None = permanent
    pub compiled_bonuses: Vec<CompiledBonus<crate::numeric::StatValue>>,
}

impl ActiveStatusEffect {
    pub fn new(effect: StatusEffect, duration_ticks: Option<u32>, stacks: u32) -> Self {
        let compiled = effect
            .bonuses
            .iter()
            .map(compile_bonus::<crate::numeric::StatValue>)
            .collect();
        Self {
            effect,
            current_stacks: stacks,
            duration_ticks,
            compiled_bonuses: compiled,
        }
    }
}

/// Manages active status effects and provides an overlaid `StatResolver`.
pub struct StatusEffectManager {
    pub active_effects: Vec<ActiveStatusEffect>,
    dirty: bool,
    active_resolver: Option<StatResolver>,
}

impl Default for StatusEffectManager {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusEffectManager {
    /// Creates a new, empty StatusEffectManager.
    pub fn new() -> Self {
        Self {
            active_effects: Vec::new(),
            dirty: true,
            active_resolver: None,
        }
    }

    /// Adds or updates a status effect.
    pub fn add_status_effect(&mut self, effect: StatusEffect, duration_ticks: Option<u32>, stacks: u32) {
        let effect_id = effect.id.clone();

        if effect.stack_behavior != StackBehavior::Independent {
            if let Some(existing) = self
                .active_effects
                .iter_mut()
                .find(|s| s.effect.id == effect_id)
            {
                match existing.effect.stack_behavior {
                    StackBehavior::Refresh => {
                        existing.duration_ticks = duration_ticks;
                    }
                    StackBehavior::Accumulate { reset_duration } => {
                        existing.current_stacks =
                            (existing.current_stacks + stacks).min(existing.effect.max_stacks);
                        if reset_duration {
                            existing.duration_ticks = duration_ticks;
                        }
                    }
                    _ => {}
                  }
                self.dirty = true;
                return;
            }
        }

        // If independent or not found, add as new
        let stacks = stacks.min(effect.max_stacks).max(1);
        self.active_effects
            .push(ActiveStatusEffect::new(effect, duration_ticks, stacks));
        self.dirty = true;
    }

    /// Advances the duration of all active status effects by one tick.
    /// Removes expired effects.
    pub fn tick(&mut self) {
        let mut expired = false;

        self.active_effects.retain_mut(|status| {
            if let Some(ref mut ticks) = status.duration_ticks {
                if *ticks > 0 {
                    *ticks -= 1;
                    if *ticks == 0 {
                        expired = true;
                        return false; // Remove
                    }
                    true // Keep
                } else {
                    expired = true;
                    false // Remove (was already 0)
                }
            } else {
                true // Permanent, keep
            }
        });

        if expired {
            self.dirty = true;
        }
    }

    /// Removes a status effect by ID.
    pub fn remove_status_effect(&mut self, effect_id: &str) {
        let original_len = self.active_effects.len();
        self.active_effects.retain(|s| s.effect.id != effect_id);
        if self.active_effects.len() != original_len {
            self.dirty = true;
        }
    }

    /// Returns a `StatResolver` with all active status effects applied as overlays.
    /// Reuses the cached resolver if no status effects have changed.
    pub fn get_active_resolver<'a>(
        &'a mut self,
        base_resolver: &StatResolver,
    ) -> &'a mut StatResolver {
        if self.dirty || self.active_resolver.is_none() {
            let mut fork = base_resolver.fork();

            for status in &self.active_effects {
                // Apply bonuses multiple times based on stacks
                for _ in 0..status.current_stacks {
                    apply_compiled_bonuses(&mut fork, &status.compiled_bonuses);
                }
            }

            self.active_resolver = Some(fork);
            self.dirty = false;
        }

        self.active_resolver.as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::StatContext;
    use crate::numeric::StatNumeric;
    use crate::source::ConstantSource;
    use crate::stat_id::StatId;

    #[test]
    fn test_status_effect_manager_buffs() {
        let mut base_resolver = StatResolver::new();
        let atk_id = StatId::from("ATK");
        base_resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

        let mut status_manager = StatusEffectManager::new();

        // Check base
        let context = StatContext::new();
        let atk = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&atk_id, &context)
            .unwrap();
        assert_eq!(atk.value.to_f64(), 100.0);

        // Add buff: +50 ATK
        let buff = StatusEffect {
            id: "BUFF_WARCRY".to_string(),
            name: "Warcry".to_string(),
            bonuses: vec![Bonus::add(atk_id.clone())
                .flat(50.0)
                .in_phase(crate::transform::TransformPhase::Additive)],
            max_stacks: 1,
            stack_behavior: StackBehavior::Refresh,
        };

        status_manager.add_status_effect(buff, Some(2), 1); // Lasts 2 ticks

        // Check buffed
        let atk = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&atk_id, &context)
            .unwrap();
        assert_eq!(atk.value.to_f64(), 150.0);

        // Tick 1
        status_manager.tick();
        let atk = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&atk_id, &context)
            .unwrap();
        assert_eq!(atk.value.to_f64(), 150.0); // Still active

        // Tick 2 (expires)
        status_manager.tick();
        let atk = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&atk_id, &context)
            .unwrap();
        assert_eq!(atk.value.to_f64(), 100.0); // Back to normal
    }

    #[test]
    fn test_status_effect_manager_accumulation() {
        let mut base_resolver = StatResolver::new();
        let def_id = StatId::from("DEF");
        base_resolver.register_source(def_id.clone(), Box::new(ConstantSource(50.0)));

        let mut status_manager = StatusEffectManager::new();
        let context = StatContext::new();

        let debuff = StatusEffect {
            id: "SUNDER_ARMOR".to_string(),
            name: "Sunder Armor".to_string(),
            bonuses: vec![Bonus::add(def_id.clone())
                .flat(-10.0)
                .in_phase(crate::transform::TransformPhase::Additive)],
            max_stacks: 5,
            stack_behavior: StackBehavior::Accumulate {
                reset_duration: true,
            },
        };

        // Stack 1
        status_manager.add_status_effect(debuff.clone(), Some(3), 1);
        let def = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&def_id, &context)
            .unwrap();
        assert_eq!(def.value.to_f64(), 40.0);

        // Stack 2
        status_manager.add_status_effect(debuff.clone(), Some(3), 1);
        let def = status_manager
            .get_active_resolver(&base_resolver)
            .resolve(&def_id, &context)
            .unwrap();
        assert_eq!(def.value.to_f64(), 30.0); // -20 total
    }
}
