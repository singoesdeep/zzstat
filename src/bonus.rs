//! Bonus system module.
//!
//! Provides a declarative API for defining bonuses that compile into
//! zzstat transforms. All branching happens during compilation, ensuring
//! zero branching during stat resolution.

use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use crate::transform::{
    AdditiveTransform, ClampTransform, MultiplicativeTransform, ScalingTransform, StackRule,
    StatTransform, TransformPhase,
};
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

/// Bonus action type.
///
/// Strongly typed actions to prevent invalid combinations like Override with Percent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BonusAction {
    /// Add a flat numeric value.
    AddFlat { value: f64 },
    /// Scale from another stat (adds source_value * factor to target).
    ScaleFrom { source: StatId, factor: f64 },
    /// Multiply by a multiplier (e.g., 1.2 for +20%).
    Multiply { multiplier: f64 },
    /// Override the stat to an absolute value.
    Override { value: f64 },
    /// Clamp to a minimum value.
    ClampMin { value: f64 },
    /// Clamp to a maximum value.
    ClampMax { value: f64 },
}

/// A bonus definition.
///
/// This is the declarative form that game code uses to define bonuses.
/// It must be compiled into a `CompiledBonus` before being applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bonus {
    /// The target stat ID.
    pub target: StatId,
    /// The action to perform.
    pub action: BonusAction,
    /// The phase in which to apply this bonus.
    pub phase: TransformPhase,
}

/// Builder for additive bonuses.
pub struct AddBonusBuilder {
    target: StatId,
}

/// Builder for scaling bonuses.
pub struct ScaleBonusBuilder {
    target: StatId,
    source: StatId,
}

/// Builder for multiplicative bonuses.
pub struct MulBonusBuilder {
    target: StatId,
}

/// Builder for additive bonuses with value set.
pub struct AddBonusBuilderWithValue {
    target: StatId,
    value: f64,
}

/// Builder for scaling bonuses with value set.
pub struct ScaleBonusBuilderWithFactor {
    target: StatId,
    source: StatId,
    factor: f64,
}

/// Builder for multiplicative bonuses with value set.
pub struct MulBonusBuilderWithValue {
    target: StatId,
    multiplier: f64,
}

impl Bonus {
    /// Create a new additive bonus builder.
    pub fn add(target: StatId) -> AddBonusBuilder {
        AddBonusBuilder { target }
    }

    /// Create a new scaling bonus builder (e.g. ATK scales from STR).
    pub fn scale(target: StatId, source: StatId) -> ScaleBonusBuilder {
        ScaleBonusBuilder { target, source }
    }

    /// Create a new multiplicative bonus builder.
    pub fn mul(target: StatId) -> MulBonusBuilder {
        MulBonusBuilder { target }
    }

    /// Create a new override bonus.
    pub fn r#override(target: StatId, value: f64) -> OverrideBonusBuilder {
        OverrideBonusBuilder { target, value }
    }

    /// Create a new clamp minimum bonus.
    pub fn clamp_min(target: StatId, value: f64) -> ClampMinBonusBuilder {
        ClampMinBonusBuilder { target, value }
    }

    /// Create a new clamp maximum bonus.
    pub fn clamp_max(target: StatId, value: f64) -> ClampMaxBonusBuilder {
        ClampMaxBonusBuilder { target, value }
    }
}

impl AddBonusBuilder {
    /// Set a flat value for the additive bonus.
    pub fn flat(self, value: f64) -> AddBonusBuilderWithValue {
        AddBonusBuilderWithValue {
            target: self.target,
            value,
        }
    }
}

impl ScaleBonusBuilder {
    /// Set the scaling factor.
    pub fn factor(self, factor: f64) -> ScaleBonusBuilderWithFactor {
        ScaleBonusBuilderWithFactor {
            target: self.target,
            source: self.source,
            factor,
        }
    }
}

impl MulBonusBuilder {
    /// Set a percentage value for the multiplicative bonus.
    ///
    /// The percentage is converted to a multiplier.
    /// For example, 0.20 means multiply by 1.20 (20% increase).
    pub fn percent(self, value: f64) -> MulBonusBuilderWithValue {
        MulBonusBuilderWithValue {
            target: self.target,
            multiplier: 1.0 + value,
        }
    }

    /// Set a direct multiplier value.
    pub fn multiplier(self, multiplier: f64) -> MulBonusBuilderWithValue {
        MulBonusBuilderWithValue {
            target: self.target,
            multiplier,
        }
    }
}

impl AddBonusBuilderWithValue {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::AddFlat { value: self.value },
            phase,
        }
    }
}

impl ScaleBonusBuilderWithFactor {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::ScaleFrom {
                source: self.source,
                factor: self.factor,
            },
            phase,
        }
    }
}

impl MulBonusBuilderWithValue {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::Multiply {
                multiplier: self.multiplier,
            },
            phase,
        }
    }
}

/// Builder for override bonuses.
pub struct OverrideBonusBuilder {
    target: StatId,
    value: f64,
}

impl OverrideBonusBuilder {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::Override { value: self.value },
            phase,
        }
    }
}

/// Builder for clamp minimum bonuses.
pub struct ClampMinBonusBuilder {
    target: StatId,
    value: f64,
}

impl ClampMinBonusBuilder {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::ClampMin { value: self.value },
            phase,
        }
    }
}

/// Builder for clamp maximum bonuses.
pub struct ClampMaxBonusBuilder {
    target: StatId,
    value: f64,
}

impl ClampMaxBonusBuilder {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            action: BonusAction::ClampMax { value: self.value },
            phase,
        }
    }
}

/// A compiled bonus that can be applied to a resolver.
#[derive(Debug, Clone)]
pub struct CompiledBonus<N: StatNumeric> {
    pub stat: StatId,
    pub phase: TransformPhase,
    pub stack_rule: StackRule,
    transform_data: TransformData,
    _phantom: std::marker::PhantomData<N>,
}

#[derive(Debug, Clone)]
enum TransformData {
    AdditiveFlat(f64),
    ScaleFrom(StatId, f64),
    Multiplicative(f64),
    Override(f64),
    ClampMin(f64),
    ClampMax(f64),
}

/// Compile a bonus into a compiled bonus.
pub fn compile_bonus<N: StatNumeric>(bonus: &Bonus) -> CompiledBonus<N> {
    let (transform_data, stack_rule) = match &bonus.action {
        BonusAction::AddFlat { value } => {
            (TransformData::AdditiveFlat(*value), StackRule::Additive)
        }
        BonusAction::ScaleFrom { source, factor } => (
            TransformData::ScaleFrom(source.clone(), *factor),
            StackRule::Additive,
        ),
        BonusAction::Multiply { multiplier } => (
            TransformData::Multiplicative(*multiplier),
            StackRule::Multiplicative,
        ),
        BonusAction::Override { value } => (TransformData::Override(*value), StackRule::Override),
        BonusAction::ClampMin { value } => (TransformData::ClampMin(*value), StackRule::MinMax),
        BonusAction::ClampMax { value } => (TransformData::ClampMax(*value), StackRule::MinMax),
    };

    CompiledBonus {
        stat: bonus.target.clone(),
        phase: bonus.phase,
        stack_rule,
        transform_data,
        _phantom: std::marker::PhantomData,
    }
}

impl<N: StatNumeric> CompiledBonus<N> {
    fn to_transform(&self) -> Box<dyn StatTransform> {
        match &self.transform_data {
            TransformData::AdditiveFlat(value) => Box::new(AdditiveTransform::new(*value)),
            TransformData::ScaleFrom(source, factor) => {
                Box::new(ScalingTransform::new(source.clone(), *factor))
            }
            TransformData::Multiplicative(multiplier) => {
                Box::new(MultiplicativeTransform::new(*multiplier))
            }
            TransformData::Override(value) => Box::new(OverrideTransform::new(*value)),
            TransformData::ClampMin(min_value) => {
                Box::new(ClampTransform::with_min(StatValue::from_f64(*min_value)))
            }
            TransformData::ClampMax(max_value) => {
                Box::new(ClampTransform::with_max(StatValue::from_f64(*max_value)))
            }
        }
    }
}

#[inline]
pub fn apply_compiled_bonus<N: StatNumeric>(
    resolver: &mut crate::resolver::StatResolver,
    compiled: &CompiledBonus<N>,
) {
    resolver.register_transform_with_rule(
        compiled.stat.clone(),
        compiled.phase,
        compiled.stack_rule,
        compiled.to_transform(),
    );
}

pub fn apply_compiled_bonuses<N: StatNumeric>(
    resolver: &mut crate::resolver::StatResolver,
    compiled: &[CompiledBonus<N>],
) {
    for bonus in compiled {
        apply_compiled_bonus(resolver, bonus);
    }
}

#[derive(Clone)]
struct OverrideTransform {
    absolute_value: f64,
}

impl OverrideTransform {
    fn new(absolute_value: f64) -> Self {
        Self { absolute_value }
    }
}

impl StatTransform for OverrideTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn phase(&self) -> TransformPhase {
        TransformPhase::Additive
    }

    fn apply(
        &self,
        _input: StatValue,
        _dependencies: &FxHashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        Ok(StatValue::from_f64(self.absolute_value))
    }

    fn description(&self) -> String {
        format!("override({:.2})", self.absolute_value)
    }
}
