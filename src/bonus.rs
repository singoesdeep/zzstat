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
    AdditiveTransform, ClampTransform, MultiplicativeTransform, StackRule, StatTransform,
    TransformPhase,
};
use std::collections::HashMap;

/// Bonus operation type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BonusOp {
    /// Add a flat or percentage value.
    Add,
    /// Multiply by a percentage.
    Multiply,
    /// Override the stat to an absolute value.
    Override,
    /// Clamp to a minimum value.
    ClampMin,
    /// Clamp to a maximum value.
    ClampMax,
}

/// Bonus value type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BonusValue {
    /// Flat numeric value.
    Flat(f64),
    /// Percentage value (e.g., 0.10 for 10%).
    Percent(f64),
}

/// A bonus definition.
///
/// This is the declarative form that game code uses to define bonuses.
/// It must be compiled into a `CompiledBonus` before being applied.
#[derive(Debug, Clone)]
pub struct Bonus {
    /// The target stat ID.
    pub target: StatId,
    /// The operation to perform.
    pub operation: BonusOp,
    /// The value for the operation.
    pub value: BonusValue,
    /// The phase in which to apply this bonus.
    pub phase: TransformPhase,
}

/// Builder for additive bonuses.
pub struct AddBonusBuilder {
    target: StatId,
}

/// Builder for multiplicative bonuses.
pub struct MulBonusBuilder {
    target: StatId,
}

/// Builder for additive bonuses with value set.
pub struct AddBonusBuilderWithValue {
    target: StatId,
    value: BonusValue,
}

/// Builder for multiplicative bonuses with value set.
pub struct MulBonusBuilderWithValue {
    target: StatId,
    value: BonusValue,
}

impl Bonus {
    /// Create a new additive bonus builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::bonus::Bonus;
    /// use zzstat::StatId;
    /// use zzstat::transform::TransformPhase;
    ///
    /// let hp_id = StatId::from_str("HP");
    /// let bonus = Bonus::add(hp_id)
    ///     .flat(50.0)
    ///     .in_phase(TransformPhase::Custom(3));
    /// ```
    pub fn add(target: StatId) -> AddBonusBuilder {
        AddBonusBuilder { target }
    }

    /// Create a new multiplicative bonus builder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::bonus::Bonus;
    /// use zzstat::StatId;
    /// use zzstat::transform::TransformPhase;
    ///
    /// let atk_id = StatId::from_str("ATK");
    /// let bonus = Bonus::mul(atk_id)
    ///     .percent(0.20)
    ///     .in_phase(TransformPhase::Custom(3));
    /// ```
    pub fn mul(target: StatId) -> MulBonusBuilder {
        MulBonusBuilder { target }
    }

    /// Create a new override bonus.
    ///
    /// Override bonuses set the stat to an absolute value, ignoring
    /// the input value. They are applied first in their phase.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::bonus::Bonus;
    /// use zzstat::StatId;
    /// use zzstat::transform::TransformPhase;
    ///
    /// let hp_id = StatId::from_str("HP");
    /// let bonus = Bonus::r#override(hp_id, 500.0)
    ///     .in_phase(TransformPhase::Custom(4));
    /// ```
    pub fn r#override(target: StatId, value: f64) -> OverrideBonusBuilder {
        OverrideBonusBuilder { target, value }
    }

    /// Create a new clamp minimum bonus.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::bonus::Bonus;
    /// use zzstat::StatId;
    /// use zzstat::transform::TransformPhase;
    ///
    /// let hp_id = StatId::from_str("HP");
    /// let bonus = Bonus::clamp_min(hp_id, 100.0)
    ///     .in_phase(TransformPhase::Final);
    /// ```
    pub fn clamp_min(target: StatId, value: f64) -> ClampMinBonusBuilder {
        ClampMinBonusBuilder { target, value }
    }

    /// Create a new clamp maximum bonus.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::bonus::Bonus;
    /// use zzstat::StatId;
    /// use zzstat::transform::TransformPhase;
    ///
    /// let crit_id = StatId::from_str("CRIT_CHANCE");
    /// let bonus = Bonus::clamp_max(crit_id, 0.75)
    ///     .in_phase(TransformPhase::Final);
    /// ```
    pub fn clamp_max(target: StatId, value: f64) -> ClampMaxBonusBuilder {
        ClampMaxBonusBuilder { target, value }
    }
}

impl AddBonusBuilder {
    /// Set a flat value for the additive bonus.
    pub fn flat(self, value: f64) -> AddBonusBuilderWithValue {
        AddBonusBuilderWithValue {
            target: self.target,
            value: BonusValue::Flat(value),
        }
    }

    /// Set a percentage value for the additive bonus.
    ///
    /// The percentage is applied to the current value and added.
    /// For example, 0.10 means add 10% of the current value.
    pub fn percent(self, value: f64) -> AddBonusBuilderWithValue {
        AddBonusBuilderWithValue {
            target: self.target,
            value: BonusValue::Percent(value),
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
            value: BonusValue::Percent(value),
        }
    }
}

impl AddBonusBuilderWithValue {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            operation: BonusOp::Add,
            value: self.value,
            phase,
        }
    }
}

impl MulBonusBuilderWithValue {
    /// Set the phase for this bonus.
    pub fn in_phase(self, phase: TransformPhase) -> Bonus {
        Bonus {
            target: self.target,
            operation: BonusOp::Multiply,
            value: self.value,
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
            operation: BonusOp::Override,
            value: BonusValue::Flat(self.value),
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
            operation: BonusOp::ClampMin,
            value: BonusValue::Flat(self.value),
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
            operation: BonusOp::ClampMax,
            value: BonusValue::Flat(self.value),
            phase,
        }
    }
}

/// A compiled bonus that can be applied to a resolver.
///
/// This is the compiled form of a `Bonus`, containing a fully constructed
/// transform that requires no branching during stat resolution.
#[derive(Debug, Clone)]
pub struct CompiledBonus<N: StatNumeric> {
    /// The target stat ID.
    pub stat: StatId,
    /// The phase in which to apply this bonus.
    pub phase: TransformPhase,
    /// The stack rule for this transform.
    pub stack_rule: StackRule,
    /// The transform data (stored as enum for cloning).
    transform_data: TransformData,
    /// Phantom data to track the numeric type (for type safety).
    _phantom: std::marker::PhantomData<N>,
}

/// Internal enum to store transform data in a cloneable way.
#[derive(Debug, Clone)]
enum TransformData {
    AdditiveFlat(f64),
    AdditivePercent(StatId, f64),
    Multiplicative(f64),
    Override(f64),
    ClampMin(f64),
    ClampMax(f64),
}

/// Compile a bonus into a compiled bonus.
///
/// This function performs all branching and matching, producing a
/// `CompiledBonus` that can be applied without any branching during
/// stat resolution.
///
/// # Arguments
///
/// * `bonus` - The bonus to compile
///
/// # Returns
///
/// A `CompiledBonus` containing the appropriate transform.
///
/// # Examples
///
/// ```rust
/// use zzstat::bonus::{Bonus, compile_bonus};
/// use zzstat::StatId;
/// use zzstat::transform::TransformPhase;
///
/// let hp_id = StatId::from_str("HP");
/// let bonus = Bonus::add(hp_id)
///     .flat(50.0)
///     .in_phase(TransformPhase::Custom(3));
///
/// let compiled = compile_bonus::<f64>(&bonus);
/// ```
pub fn compile_bonus<N: StatNumeric>(bonus: &Bonus) -> CompiledBonus<N> {
    let (transform_data, stack_rule) = match bonus.operation {
        BonusOp::Add => match bonus.value {
            BonusValue::Flat(value) => (TransformData::AdditiveFlat(value), StackRule::Additive),
            BonusValue::Percent(percent) => (
                TransformData::AdditivePercent(bonus.target.clone(), percent),
                StackRule::Additive,
            ),
        },
        BonusOp::Multiply => {
            let multiplier = match bonus.value {
                BonusValue::Percent(percent) => 1.0 + percent,
                BonusValue::Flat(v) => v,
            };
            (
                TransformData::Multiplicative(multiplier),
                StackRule::Multiplicative,
            )
        }
        BonusOp::Override => {
            let value = match bonus.value {
                BonusValue::Flat(v) => v,
                BonusValue::Percent(_) => bonus.value.to_f64(),
            };
            (TransformData::Override(value), StackRule::Override)
        }
        BonusOp::ClampMin => {
            let min_value = match bonus.value {
                BonusValue::Flat(v) => v,
                BonusValue::Percent(_) => bonus.value.to_f64(),
            };
            (TransformData::ClampMin(min_value), StackRule::MinMax)
        }
        BonusOp::ClampMax => {
            let max_value = match bonus.value {
                BonusValue::Flat(v) => v,
                BonusValue::Percent(_) => bonus.value.to_f64(),
            };
            (TransformData::ClampMax(max_value), StackRule::MinMax)
        }
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
    /// Create a Box<dyn StatTransform> from the stored transform data.
    fn to_transform(&self) -> Box<dyn StatTransform> {
        match &self.transform_data {
            TransformData::AdditiveFlat(value) => Box::new(AdditiveTransform::new(*value)),
            TransformData::AdditivePercent(dep, percent) => {
                Box::new(PercentAdditiveTransform::new(dep.clone(), *percent))
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

/// Apply a compiled bonus to a resolver.
///
/// This function registers the compiled bonus's transform to the resolver
/// with the appropriate phase and stack rule. It contains no branching logic.
///
/// # Arguments
///
/// * `resolver` - The resolver to apply the bonus to
/// * `compiled` - The compiled bonus to apply
///
/// # Examples
///
/// ```rust
/// use zzstat::bonus::{Bonus, compile_bonus, apply_compiled_bonus};
/// use zzstat::{StatId, StatResolver};
/// use zzstat::transform::TransformPhase;
///
/// let mut resolver = StatResolver::new();
/// let hp_id = StatId::from_str("HP");
/// let bonus = Bonus::add(hp_id)
///     .flat(50.0)
///     .in_phase(TransformPhase::Custom(3));
///
/// let compiled = compile_bonus::<f64>(&bonus);
/// apply_compiled_bonus(&mut resolver, &compiled);
/// ```
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

/// Apply multiple compiled bonuses to a resolver.
///
/// # Arguments
///
/// * `resolver` - The resolver to apply bonuses to
/// * `compiled` - The compiled bonuses to apply
///
/// # Examples
///
/// ```rust
/// use zzstat::bonus::{Bonus, compile_bonus, apply_compiled_bonuses};
/// use zzstat::{StatId, StatResolver};
/// use zzstat::transform::TransformPhase;
///
/// let mut resolver = StatResolver::new();
/// let hp_id = StatId::from_str("HP");
/// let bonuses = vec![
///     Bonus::add(hp_id.clone()).flat(50.0).in_phase(TransformPhase::Custom(3)),
///     Bonus::mul(hp_id).percent(0.10).in_phase(TransformPhase::Custom(3)),
/// ];
///
/// let compiled: Vec<_> = bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();
/// apply_compiled_bonuses(&mut resolver, &compiled);
/// ```
pub fn apply_compiled_bonuses<N: StatNumeric>(
    resolver: &mut crate::resolver::StatResolver,
    compiled: &[CompiledBonus<N>],
) {
    for bonus in compiled {
        apply_compiled_bonus(resolver, bonus);
    }
}

// Custom transforms

/// A transform that adds a percentage of the current value.
///
/// This is used for additive percent bonuses (e.g., +10% HP).
/// It depends on the stat itself to read the current value.
struct PercentAdditiveTransform {
    dependency: StatId,
    percent: f64,
}

impl PercentAdditiveTransform {
    fn new(dependency: StatId, percent: f64) -> Self {
        Self {
            dependency,
            percent,
        }
    }
}

impl StatTransform for PercentAdditiveTransform {
    fn depends_on(&self) -> Vec<StatId> {
        vec![self.dependency.clone()]
    }

    fn phase(&self) -> TransformPhase {
        TransformPhase::Additive
    }

    fn apply(
        &self,
        input: StatValue,
        dependencies: &HashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        let dep_value = dependencies
            .get(&self.dependency)
            .ok_or_else(|| StatError::MissingDependency(self.dependency.clone()))?;
        // Add (current_value * percent) to input
        let bonus = *dep_value * StatValue::from_f64(self.percent);
        Ok(input + bonus)
    }

    fn description(&self) -> String {
        format!("+{:.1}% (additive)", self.percent * 100.0)
    }
}

impl Clone for PercentAdditiveTransform {
    fn clone(&self) -> Self {
        Self {
            dependency: self.dependency.clone(),
            percent: self.percent,
        }
    }
}

/// A transform that overrides the stat to an absolute value.
///
/// This transform ignores the input value completely and returns
/// the absolute value. It is used for Override bonuses.
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
        TransformPhase::Additive // Default, will be overridden by phase in CompiledBonus
    }

    fn apply(
        &self,
        _input: StatValue,
        _dependencies: &HashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        // Always return the absolute value, completely ignoring input
        Ok(StatValue::from_f64(self.absolute_value))
    }

    fn description(&self) -> String {
        format!("override({:.2})", self.absolute_value)
    }
}

// Helper implementation for BonusValue
impl BonusValue {
    fn to_f64(self) -> f64 {
        match self {
            BonusValue::Flat(v) => v,
            BonusValue::Percent(v) => v,
        }
    }
}
