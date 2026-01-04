//! # zzstat - Deterministic, Hardcode-Free MMORPG Stat Engine
//!
//! A stat calculation engine designed for MMORPGs that provides:
//! - **Deterministic** stat resolution (same input → same output)
//! - **Hardcode-free** design (no built-in stat names like "HP" or "ATK")
//! - **Event-driven** resolution (only resolves when invalidated)
//! - **Phase-based** transformation pipeline
//!
//! ## Core Concepts
//!
//! ### Stat Pipeline
//!
//! Stats flow through a simple pipeline:
//!
//! ```text
//! [StatSource] → [StatTransform] → [ResolvedStat]
//! ```
//!
//! 1. **Sources** produce base values (additive)
//! 2. **Transforms** modify values (can depend on other stats)
//! 3. **ResolvedStat** contains the final value with full breakdown
//!
//! ### Key Features
//!
//! - **Dependency Graph**: Automatically resolves dependencies in correct order
//! - **Cycle Detection**: Prevents circular dependencies
//! - **Caching**: Resolved stats are cached until invalidated
//! - **Context-Aware**: Supports conditional calculations via `StatContext`
//! - **Debug-Friendly**: Full breakdown of sources and transforms
//!
//! ## Example
//!
//! ```rust
//! use zzstat::*;
//! use zzstat::source::ConstantSource;
//! use zzstat::transform::MultiplicativeTransform;
//!
//! let mut resolver = StatResolver::new();
//! let hp_id = StatId::from_str("HP");
//!
//! // Register sources (additive)
//! resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
//! resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
//!
//! // Register transform
//! resolver.register_transform(hp_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));
//!
//! // Resolve
//! let context = StatContext::new();
//! let resolved = resolver.resolve(&hp_id, &context).unwrap();
//! assert_eq!(resolved.value, 225.0); // (100 + 50) * 1.5
//! ```
//!
//! ## Modules
//!
//! - [`stat_id`] - Stat identifier type
//! - [`source`] - Stat sources (produce base values)
//! - [`transform`] - Stat transforms (modify values)
//! - [`resolver`] - Main stat resolver
//! - [`resolved`] - Resolved stat results
//! - [`context`] - Context for conditional calculations
//! - [`graph`] - Dependency graph management
//! - [`error`] - Error types

pub mod bonus;
pub mod context;
pub mod error;
pub mod graph;
pub mod numeric;
pub mod resolved;
pub mod resolver;
pub mod source;
pub mod stat_id;
pub mod transform;

// Re-export main types for convenience
pub use context::StatContext;
pub use error::StatError;
pub use resolved::ResolvedStat;
pub use resolver::StatResolver;
pub use stat_id::StatId;

// Re-export common sources and transforms
pub use source::{ConstantSource, MapSource, StatSource};
pub use transform::{
    AdditiveTransform, ClampTransform, ConditionalTransform, MultiplicativeTransform,
    ScalingTransform, StackRule, StatTransform, TransformEntry, TransformPhase,
};

// Re-export numeric types
#[cfg(feature = "fixed-point")]
pub use numeric::FixedPoint;
pub use numeric::{StatNumeric, StatValue};

// Re-export bonus types
pub use bonus::{
    apply_compiled_bonus, apply_compiled_bonuses, compile_bonus, Bonus, BonusOp, BonusValue,
    CompiledBonus,
};
