# zzstat - Hardcode-Free MMORPG Stat Engine

A highly optimized, completely data-driven, and deterministic stat calculation engine designed specifically for RPG and MMORPG servers. 

**zzstat** provides everything you need to build complex game mechanics without writing a single line of hardcoded game logic. With zero I/O, no async primitives, and an incredibly fast `O(1)` copy-on-write `fork()` architecture, you can evaluate thousands of combat formulas and buffs concurrently with maximum performance.

## 🚀 Features at a Glance

- **JSON Data-Driven:** Define your stat formulas, buffs, combat skills, and conditional abilities entirely in JSON. Let Game Designers balance the game without touching Rust code.
- **Dependency Graph Resolution:** Automatically resolves complex dependency chains (`ATK = STR * 2`, `DAMAGE = ATK - DEF`) with topological sorting and cycle detection.
- **Resource Pools & DoT/HoT:** Track stateful values like Current HP/MP, apply Damage over Time (Poison), and trigger custom Events (like `DEATH`).
- **Temporary Status Effects (Buffs & Debuffs):** A `StatusManager` that seamlessly applies temporary buffs using `fork()`, ensuring the character's base stats are never mutated. Supports stacking behaviors (`Refresh`, `Accumulate`, `Independent`).
- **Combat Engine (AST):** A fully JSON-driven Combat Abstract Syntax Tree (AST). Calculate complex damages involving `Chance` nodes (Crit, Dodge, Block) deterministically.
- **AI-Ready Guidelines:** Includes `AI_PROMPT.md` out-of-the-box so you can instantly generate perfectly valid JSON files using AI assistants like ChatGPT, Claude, or Copilot.
- **Modular & High Performance:** Fully decoupled `StatRegistry` and `StatResolver` using `rustc_hash::FxHashMap` for blazingly fast `O(1)` operations.

---

## 🛠️ Core Modules

### 1. StatTemplate (Data-Driven Architecture)
Forget writing manual transforms in Rust. Load your entire class, monster, or item stat logic directly from a JSON string.

```json
{
  "name": "Knight Class",
  "transforms": [
    {
      "stat": "DEF",
      "phase": "additive",
      "rule": "additive",
      "type": "Conditional",
      "condition": { "operator": "Equals", "key": "STANCE", "value": "DEFENSIVE" },
      "transform": { "type": "Additive", "value": 50.0 }
    }
  ]
}
```

### 2. ResourcePool (State Management)
RPG games need to track current states. `ResourcePool` handles Current HP/MP, bounds it to a `StatResolver` stat (like `MAX_HP`), processes ticks for DoTs, and emits events when thresholds are hit.

```rust
// Poison DoT (-20 HP/tick for 3 ticks)
hp_pool.add_effect(TimeEffect {
    name: "Poison".to_string(),
    amount_per_tick: -20.0,
    ticks_remaining: 3,
});

// Emits "DEATH" event if HP falls to 0
hp_pool.add_trigger(ThresholdTrigger {
    condition: TriggerCondition::Empty,
    event_name: "DEATH".to_string(),
});
```

### 3. StatusManager (Temporary Buffs/Debuffs)
Adding a temporary buff shouldn't permanently alter the base stats. `zzstat` uses an `O(1)` copy-on-write `fork()` method to overlay stats dynamically.

```rust
let buff = StatusEffect {
    id: "WARCRY".to_string(),
    name: "Warcry".to_string(),
    bonuses: vec![Bonus::add(atk_id).flat(50.0)],
    max_stacks: 1,
    stack_behavior: StackBehavior::Refresh,
};

// Add 50 ATK for 3 ticks
status_manager.add_status(buff, Some(3), 1);

// Use the active resolver safely
let current_atk = status_manager.get_active_resolver(&base_resolver).resolve(&atk_id, &ctx);
```

### 4. CombatEngine (AST Formula Evaluation)
Evaluate complex attack commands defined entirely in JSON. The `CombatEngine` reads the AST and calculates the damage based on the Attacker and Defender's stats. Random chances (Crit/Dodge) rely on a closure, making tests 100% deterministic!

```json
{
  "name": "Sword Slash",
  "expression": {
    "type": "Chance",
    "chance_expr": { "type": "Stat", "target": "defender", "stat": "DODGE" },
    "success_expr": { "type": "Constant", "value": 0.0 },
    "fail_expr": {
      "type": "Subtract",
      "left": { "type": "Stat", "target": "attacker", "stat": "ATK" },
      "right": { "type": "Stat", "target": "defender", "stat": "DEF" }
    }
  }
}
```

---

## ⚡ Performance & Determinism
In MMORPG architecture, backend loops calculate stats for thousands of entities per second.
1. **Zero Allocations on Runtime Graph:** Calculations trace predefined paths using robust caching (`FxHashMap`).
2. **Copy-on-Write Forks:** Applying a buff creates an `Arc` overlay in nanoseconds.
3. **Fixed-Point Arithmetic (Optional):** Ensure cross-platform floating-point precision issues never desync your lockstep gameplay.

---

## 📚 Examples
Explore the `/examples` directory for comprehensive guides on how to use `zzstat`:
- `01_template_loader.rs`
- `02_resource_pool.rs`
- `03_status_effects.rs`
- `04_combat_engine.rs`

Run them with:
```bash
cargo run --example 01_template_loader
cargo run --example 02_resource_pool
```

## 🤝 Using AI to Create Content
You can use ChatGPT, Claude, or any LLM to generate perfectly formatted stat and combat JSONs for `zzstat`. Simply copy the contents of `AI_PROMPT.md` and give it to your AI assistant.

---

## License
This project is licensed under the MIT License.
