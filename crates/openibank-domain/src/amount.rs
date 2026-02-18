//! Fixed-point IUSD amount arithmetic — no floating point, ever.
//!
//! 1 IUSD = 1_000_000 microdollars (6 decimal places).
//! Stored as `u128` to prevent overflow on large financial operations.

use std::fmt;
use std::iter::Sum;
use std::ops::{Add, Sub};

use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// IUSD amount stored as microdollars (6 decimal places).
///
/// # Examples
///
/// ```
/// use openibank_domain::IusdAmount;
///
/// let fifty = IusdAmount::from_dollars(50);
/// let quarter = IusdAmount::from_decimal_str("0.25").unwrap();
/// let total = fifty.checked_add(&quarter).unwrap();
/// assert_eq!(total.to_display_string(), "50.25 IUSD");
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IusdAmount(u128);

impl IusdAmount {
    /// Zero IUSD.
    pub const ZERO: Self = IusdAmount(0);
    /// Exactly 1.000000 IUSD.
    pub const ONE_DOLLAR: Self = IusdAmount(1_000_000);
    /// Maximum representable IUSD.
    pub const MAX: Self = IusdAmount(u128::MAX);

    /// Number of decimal places (6).
    const DECIMALS: u32 = 6;
    /// Microdollars per dollar.
    const SCALE: u128 = 1_000_000;

    /// Create from raw microdollar amount.
    #[inline]
    pub fn from_micros(micros: u128) -> Self {
        IusdAmount(micros)
    }

    /// Create from whole dollars (e.g., `50` → `50.000000 IUSD`).
    #[inline]
    pub fn from_dollars(dollars: u64) -> Self {
        IusdAmount((dollars as u128).saturating_mul(Self::SCALE))
    }

    /// Parse a decimal string like `"50.25"` or `"100"`.
    ///
    /// Up to 6 decimal places accepted. Extra decimals → error.
    pub fn from_decimal_str(s: &str) -> Result<Self, DomainError> {
        let s = s.trim();
        if let Some((whole, frac)) = s.split_once('.') {
            if frac.len() > Self::DECIMALS as usize {
                return Err(DomainError::InvalidAmount(format!(
                    "too many decimal places in '{}' (max 6)",
                    s
                )));
            }
            let whole_val: u128 = whole.parse().map_err(|_| {
                DomainError::InvalidAmount(format!("invalid whole part: '{}'", whole))
            })?;
            let frac_str = format!("{:0<6}", frac); // right-pad to 6 digits
            let frac_val: u128 = frac_str[..6].parse().map_err(|_| {
                DomainError::InvalidAmount(format!("invalid fractional part: '{}'", frac))
            })?;
            Ok(IusdAmount(
                whole_val.checked_mul(Self::SCALE).ok_or_else(|| {
                    DomainError::InvalidAmount("amount overflow".to_string())
                })?
                .checked_add(frac_val)
                .ok_or_else(|| DomainError::InvalidAmount("amount overflow".to_string()))?,
            ))
        } else {
            let whole_val: u128 = s.parse().map_err(|_| {
                DomainError::InvalidAmount(format!("invalid amount: '{}'", s))
            })?;
            Ok(IusdAmount(whole_val.saturating_mul(Self::SCALE)))
        }
    }

    /// Raw microdollar value.
    #[inline]
    pub fn micros(&self) -> u128 {
        self.0
    }

    /// Format as `"50.250000"` (always 6 decimal places).
    pub fn to_decimal_string(&self) -> String {
        format!("{}.{:06}", self.0 / Self::SCALE, self.0 % Self::SCALE)
    }

    /// Format as `"50.25 IUSD"` (trailing zeros stripped).
    pub fn to_display_string(&self) -> String {
        let s = self.to_decimal_string();
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        format!("{} IUSD", s)
    }

    /// Checked addition — returns `None` on overflow.
    #[inline]
    pub fn checked_add(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(IusdAmount)
    }

    /// Checked subtraction — returns `None` if result would be negative.
    #[inline]
    pub fn checked_sub(&self, rhs: &Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(IusdAmount)
    }

    /// Checked multiplication by a scalar.
    #[inline]
    pub fn checked_mul_u64(&self, factor: u64) -> Option<Self> {
        self.0.checked_mul(factor as u128).map(IusdAmount)
    }

    /// Saturating addition (caps at `MAX` rather than panicking).
    #[inline]
    pub fn saturating_add(&self, rhs: &Self) -> Self {
        IusdAmount(self.0.saturating_add(rhs.0))
    }

    /// Returns `true` if the amount is exactly zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for IusdAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_display_string())
    }
}

/// **Panics on overflow** — use `checked_add` in production financial code.
impl Add for IusdAmount {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        IusdAmount(
            self.0
                .checked_add(rhs.0)
                .expect("IusdAmount addition overflow"),
        )
    }
}

/// **Panics if result negative** — use `checked_sub` in production financial code.
impl Sub for IusdAmount {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        IusdAmount(
            self.0
                .checked_sub(rhs.0)
                .expect("IusdAmount subtraction underflow"),
        )
    }
}

impl Sum for IusdAmount {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(IusdAmount::ZERO, |acc, x| {
            acc.checked_add(&x).expect("IusdAmount sum overflow")
        })
    }
}

impl Default for IusdAmount {
    fn default() -> Self {
        IusdAmount::ZERO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_dollars() {
        assert_eq!(IusdAmount::from_dollars(50).micros(), 50_000_000);
        assert_eq!(IusdAmount::ONE_DOLLAR.micros(), 1_000_000);
    }

    #[test]
    fn test_decimal_roundtrip() {
        let cases = ["50.25", "0.000001", "1000000.123456", "0", "1"];
        for s in &cases {
            let parsed = IusdAmount::from_decimal_str(s).unwrap();
            let rendered = parsed.to_decimal_string();
            let reparsed = IusdAmount::from_decimal_str(&rendered).unwrap();
            assert_eq!(parsed, reparsed, "roundtrip failed for '{}'", s);
        }
    }

    #[test]
    fn test_display() {
        assert_eq!(IusdAmount::from_decimal_str("50.25").unwrap().to_display_string(), "50.25 IUSD");
        assert_eq!(IusdAmount::from_dollars(100).to_display_string(), "100 IUSD");
        assert_eq!(IusdAmount::ZERO.to_display_string(), "0 IUSD");
    }

    #[test]
    fn test_checked_add_overflow() {
        assert!(IusdAmount::MAX.checked_add(&IusdAmount::ONE_DOLLAR).is_none());
    }

    #[test]
    fn test_checked_sub_underflow() {
        assert!(IusdAmount::ZERO.checked_sub(&IusdAmount::ONE_DOLLAR).is_none());
    }

    #[test]
    fn test_ordering() {
        let a = IusdAmount::from_dollars(10);
        let b = IusdAmount::from_dollars(20);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(a, a);
    }

    #[test]
    fn test_sum() {
        let amounts = vec![
            IusdAmount::from_dollars(10),
            IusdAmount::from_dollars(20),
            IusdAmount::from_dollars(30),
        ];
        let total: IusdAmount = amounts.into_iter().sum();
        assert_eq!(total, IusdAmount::from_dollars(60));
    }

    #[test]
    fn test_too_many_decimals() {
        assert!(IusdAmount::from_decimal_str("1.1234567").is_err());
    }
}
