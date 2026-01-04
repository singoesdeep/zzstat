//! Numeric types for stat values.
//!
//! Provides a fixed-point numeric type for deterministic calculations
//! when the `fixed-point` feature is enabled, or uses `f64` by default.

use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "fixed-point")]
use serde::{Deserialize, Serialize};

/// Trait for numeric operations required by stat calculations.
///
/// This trait abstracts over `f64` and `FixedPoint` to allow
/// the stat system to work with either numeric backend.
pub trait StatNumeric:
    Clone
    + Copy
    + PartialEq
    + PartialOrd
    + fmt::Debug
    + fmt::Display
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Default
{
    /// Create a zero value.
    fn zero() -> Self;

    /// Create a value from an integer.
    fn from_int(i: i64) -> Self;

    /// Create a value from f64.
    fn from_f64(f: f64) -> Self;

    /// Convert to f64.
    fn to_f64(self) -> f64;

    /// Clamp the value between min and max (inclusive).
    fn clamp(self, min: Self, max: Self) -> Self;
}

#[cfg(not(feature = "fixed-point"))]
impl StatNumeric for f64 {
    fn zero() -> Self {
        0.0
    }

    fn from_int(i: i64) -> Self {
        i as f64
    }

    fn from_f64(f: f64) -> Self {
        f
    }

    fn to_f64(self) -> f64 {
        self
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        self.clamp(min, max)
    }
}

/// Fixed-point number for deterministic stat calculations.
///
/// Uses `i64` for the value and `u8` for the scale (number of decimal places).
/// For example, with scale 4, the value 12345 represents 1.2345.
///
/// # Examples
///
/// ```rust
/// use zzstat::numeric::FixedPoint;
///
/// let fp = FixedPoint::new(12345, 4);
/// assert_eq!(fp.to_f64(), 1.2345);
/// ```
#[cfg(feature = "fixed-point")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FixedPoint {
    /// The integer value (scaled by 10^scale).
    value: i64,
    /// The scale factor (number of decimal places).
    scale: u8,
}

#[cfg(feature = "fixed-point")]
impl FixedPoint {
    /// Default scale for fixed-point numbers (4 decimal places).
    pub const DEFAULT_SCALE: u8 = 4;

    /// Create a new fixed-point number.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer value (scaled by 10^scale)
    /// * `scale` - The number of decimal places (0-18)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::numeric::FixedPoint;
    ///
    /// // 1.2345 with 4 decimal places
    /// let fp = FixedPoint::new(12345, 4);
    /// ```
    pub fn new(value: i64, scale: u8) -> Self {
        Self { value, scale }
    }

    /// Create a fixed-point number from an f64.
    ///
    /// Uses the default scale (4 decimal places).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::numeric::FixedPoint;
    ///
    /// let fp = FixedPoint::from_f64(1.2345);
    /// ```
    pub fn from_f64(f: f64) -> Self {
        Self::from_f64_with_scale(f, Self::DEFAULT_SCALE)
    }

    /// Create a fixed-point number from an f64 with a specific scale.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::numeric::FixedPoint;
    ///
    /// let fp = FixedPoint::from_f64_with_scale(1.234567, 6);
    /// ```
    pub fn from_f64_with_scale(f: f64, scale: u8) -> Self {
        let multiplier = 10_i64.pow(scale as u32);
        let value = (f * multiplier as f64).round() as i64;
        Self { value, scale }
    }

    /// Convert to f64.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::numeric::FixedPoint;
    ///
    /// let fp = FixedPoint::new(12345, 4);
    /// assert!((fp.to_f64() - 1.2345).abs() < 0.0001);
    /// ```
    pub fn to_f64(self) -> f64 {
        let divisor = 10_f64.powi(self.scale as i32);
        self.value as f64 / divisor
    }

    /// Get the raw integer value.
    pub fn value(self) -> i64 {
        self.value
    }

    /// Get the scale.
    pub fn scale(self) -> u8 {
        self.scale
    }

    /// Normalize to a common scale for arithmetic operations.
    ///
    /// Returns (value1, value2, common_scale) where both values
    /// are scaled to the same factor.
    fn normalize(self, other: Self) -> (i64, i64, u8) {
        let common_scale = self.scale.max(other.scale);
        let scale_diff1 = common_scale as i32 - self.scale as i32;
        let scale_diff2 = common_scale as i32 - other.scale as i32;

        let value1 = if scale_diff1 > 0 {
            self.value * 10_i64.pow(scale_diff1 as u32)
        } else {
            self.value
        };

        let value2 = if scale_diff2 > 0 {
            other.value * 10_i64.pow(scale_diff2 as u32)
        } else {
            other.value
        };

        (value1, value2, common_scale)
    }
}

#[cfg(feature = "fixed-point")]
impl Default for FixedPoint {
    fn default() -> Self {
        Self {
            value: 0,
            scale: Self::DEFAULT_SCALE,
        }
    }
}

#[cfg(feature = "fixed-point")]
impl From<f64> for FixedPoint {
    fn from(f: f64) -> Self {
        Self::from_f64(f)
    }
}

#[cfg(feature = "fixed-point")]
impl From<FixedPoint> for f64 {
    fn from(fp: FixedPoint) -> Self {
        fp.to_f64()
    }
}

#[cfg(feature = "fixed-point")]
impl Add for FixedPoint {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let (v1, v2, scale) = self.normalize(other);
        Self {
            value: v1 + v2,
            scale,
        }
    }
}

#[cfg(feature = "fixed-point")]
impl Sub for FixedPoint {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let (v1, v2, scale) = self.normalize(other);
        Self {
            value: v1 - v2,
            scale,
        }
    }
}

#[cfg(feature = "fixed-point")]
impl Mul for FixedPoint {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        let (v1, v2, scale) = self.normalize(other);
        // Result needs to be divided by 10^scale to maintain scale
        let result = (v1 * v2) / 10_i64.pow(scale as u32);
        Self {
            value: result,
            scale,
        }
    }
}

#[cfg(feature = "fixed-point")]
impl Div for FixedPoint {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let (v1, v2, scale) = self.normalize(other);
        // Result needs to be multiplied by 10^scale to maintain scale
        let result = (v1 * 10_i64.pow(scale as u32)) / v2;
        Self {
            value: result,
            scale,
        }
    }
}

#[cfg(feature = "fixed-point")]
impl StatNumeric for FixedPoint {
    fn zero() -> Self {
        Self {
            value: 0,
            scale: Self::DEFAULT_SCALE,
        }
    }

    fn from_int(i: i64) -> Self {
        Self {
            value: i * 10_i64.pow(Self::DEFAULT_SCALE as u32),
            scale: Self::DEFAULT_SCALE,
        }
    }

    fn from_f64(f: f64) -> Self {
        Self::from_f64(f)
    }

    fn to_f64(self) -> f64 {
        self.to_f64()
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

#[cfg(feature = "fixed-point")]
impl fmt::Display for FixedPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.to_f64())
    }
}

/// Type alias for stat values.
///
/// Uses `FixedPoint` when the `fixed-point` feature is enabled,
/// otherwise uses `f64`.
#[cfg(feature = "fixed-point")]
pub type StatValue = FixedPoint;

#[cfg(not(feature = "fixed-point"))]
pub type StatValue = f64;

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "fixed-point")]
    #[test]
    fn test_fixed_point_creation() {
        let fp = FixedPoint::new(12345, 4);
        assert_eq!(fp.value(), 12345);
        assert_eq!(fp.scale(), 4);
    }

    #[cfg(feature = "fixed-point")]
    #[test]
    fn test_fixed_point_from_f64() {
        let fp = FixedPoint::from_f64(1.2345);
        assert!((fp.to_f64() - 1.2345).abs() < 0.0001);
    }

    #[cfg(feature = "fixed-point")]
    #[test]
    fn test_fixed_point_arithmetic() {
        let a = FixedPoint::from_f64(10.0);
        let b = FixedPoint::from_f64(5.0);

        assert!((a.add(b).to_f64() - 15.0).abs() < 0.0001);
        assert!((a.sub(b).to_f64() - 5.0).abs() < 0.0001);
        assert!((a.mul(b).to_f64() - 50.0).abs() < 0.1); // Multiplication has precision loss
        assert!((a.div(b).to_f64() - 2.0).abs() < 0.1); // Division has precision loss
    }

    #[cfg(feature = "fixed-point")]
    #[test]
    fn test_fixed_point_different_scales() {
        let a = FixedPoint::new(12345, 4); // 1.2345
        let b = FixedPoint::new(123456, 5); // 1.23456

        let sum = a + b;
        // Should normalize to scale 5
        assert_eq!(sum.scale(), 5);
    }

    #[test]
    fn test_stat_numeric_trait() {
        #[cfg(not(feature = "fixed-point"))]
        {
            let zero: f64 = StatNumeric::zero();
            assert_eq!(zero, 0.0);
        }

        #[cfg(feature = "fixed-point")]
        {
            let zero: FixedPoint = StatNumeric::zero();
            assert_eq!(zero.value(), 0);
        }
    }
}

