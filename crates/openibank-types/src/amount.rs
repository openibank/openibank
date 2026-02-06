//! Amount types with 18-decimal precision
//!
//! OpeniBank uses fixed-point arithmetic with i128 for amounts to ensure
//! overflow-safe operations and support for very large values.

use crate::{Currency, OpeniBankError, Result};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, Sub};

/// Standard precision for internal calculations (18 decimals)
pub const STANDARD_DECIMALS: u8 = 18;

/// The standard multiplier for 18 decimals
pub const STANDARD_MULTIPLIER: i128 = 1_000_000_000_000_000_000;

/// High-precision amount with currency
///
/// Uses i128 for the value (in smallest units) and supports up to 18 decimal places.
/// This provides:
/// - Support for very large amounts (up to ~170 undecillion)
/// - Support for negative values (for debits/credits in ledger)
/// - Safe arithmetic with overflow checking
/// - Currency-aware operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Amount {
    /// Raw value in smallest units (e.g., wei for ETH, cents*10^16 for USD)
    pub value: i128,
    /// The currency
    pub currency: Currency,
    /// Number of decimal places (typically 18)
    pub decimals: u8,
}

impl Amount {
    /// Create a new amount
    pub fn new(value: i128, currency: Currency, decimals: u8) -> Self {
        Self {
            value,
            currency,
            decimals,
        }
    }

    /// Create a zero amount
    pub fn zero(currency: Currency) -> Self {
        Self {
            value: 0,
            currency,
            decimals: STANDARD_DECIMALS,
        }
    }

    /// Create an amount from a human-readable value (e.g., 100.50)
    pub fn from_human(human_value: f64, currency: Currency) -> Self {
        let decimals = STANDARD_DECIMALS;
        let multiplier = 10i128.pow(decimals as u32);
        let value = (human_value * multiplier as f64) as i128;
        Self {
            value,
            currency,
            decimals,
        }
    }

    /// Create an amount from human value with specific decimals
    pub fn from_human_with_decimals(human_value: f64, currency: Currency, decimals: u8) -> Self {
        let multiplier = 10i128.pow(decimals as u32);
        let value = (human_value * multiplier as f64) as i128;
        Self {
            value,
            currency,
            decimals,
        }
    }

    /// Get the human-readable value
    pub fn to_human(&self) -> f64 {
        let divisor = 10i128.pow(self.decimals as u32) as f64;
        self.value as f64 / divisor
    }

    /// Check if the amount is zero
    pub fn is_zero(&self) -> bool {
        self.value == 0
    }

    /// Check if the amount is positive
    pub fn is_positive(&self) -> bool {
        self.value > 0
    }

    /// Check if the amount is negative
    pub fn is_negative(&self) -> bool {
        self.value < 0
    }

    /// Get the absolute value
    pub fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
            ..*self
        }
    }

    /// Negate the amount
    pub fn negate(&self) -> Self {
        Self {
            value: -self.value,
            ..*self
        }
    }

    /// Get the multiplier for this decimal precision
    pub fn multiplier(&self) -> i128 {
        10i128.pow(self.decimals as u32)
    }

    /// Scale this amount to a different decimal precision
    pub fn scale_to(&self, target_decimals: u8) -> Result<Self> {
        if target_decimals == self.decimals {
            return Ok(*self);
        }

        let value = if target_decimals > self.decimals {
            // Scale up (multiply)
            let diff = target_decimals - self.decimals;
            let multiplier = 10i128.pow(diff as u32);
            self.value
                .checked_mul(multiplier)
                .ok_or(OpeniBankError::AmountOverflow)?
        } else {
            // Scale down (divide with rounding)
            let diff = self.decimals - target_decimals;
            let divisor = 10i128.pow(diff as u32);
            self.value / divisor
        };

        Ok(Self {
            value,
            currency: self.currency,
            decimals: target_decimals,
        })
    }

    /// Scale to standard 18 decimals
    pub fn to_standard(&self) -> Result<Self> {
        self.scale_to(STANDARD_DECIMALS)
    }

    /// Checked addition (currencies must match)
    pub fn checked_add(self, other: Self) -> Result<Self> {
        if self.currency != other.currency {
            return Err(OpeniBankError::CurrencyMismatch {
                expected: self.currency.symbol(),
                actual: other.currency.symbol(),
            });
        }

        // Scale both to the same decimals
        let (a, b) = if self.decimals != other.decimals {
            let max_decimals = self.decimals.max(other.decimals);
            (self.scale_to(max_decimals)?, other.scale_to(max_decimals)?)
        } else {
            (self, other)
        };

        let value = a
            .value
            .checked_add(b.value)
            .ok_or(OpeniBankError::AmountOverflow)?;

        Ok(Self {
            value,
            currency: a.currency,
            decimals: a.decimals,
        })
    }

    /// Checked subtraction (currencies must match)
    pub fn checked_sub(self, other: Self) -> Result<Self> {
        if self.currency != other.currency {
            return Err(OpeniBankError::CurrencyMismatch {
                expected: self.currency.symbol(),
                actual: other.currency.symbol(),
            });
        }

        let (a, b) = if self.decimals != other.decimals {
            let max_decimals = self.decimals.max(other.decimals);
            (self.scale_to(max_decimals)?, other.scale_to(max_decimals)?)
        } else {
            (self, other)
        };

        let value = a
            .value
            .checked_sub(b.value)
            .ok_or(OpeniBankError::AmountUnderflow)?;

        Ok(Self {
            value,
            currency: a.currency,
            decimals: a.decimals,
        })
    }

    /// Checked multiplication by a scalar
    pub fn checked_mul(self, multiplier: i128) -> Result<Self> {
        let value = self
            .value
            .checked_mul(multiplier)
            .ok_or(OpeniBankError::AmountOverflow)?;
        Ok(Self { value, ..self })
    }

    /// Checked division by a scalar
    pub fn checked_div(self, divisor: i128) -> Result<Self> {
        if divisor == 0 {
            return Err(OpeniBankError::DivisionByZero);
        }
        Ok(Self {
            value: self.value / divisor,
            ..self
        })
    }

    /// Multiply by a percentage (0-100)
    pub fn percentage(self, percent: u8) -> Result<Self> {
        let value = self
            .value
            .checked_mul(percent as i128)
            .ok_or(OpeniBankError::AmountOverflow)?
            / 100;
        Ok(Self { value, ..self })
    }

    /// Multiply by basis points (0-10000, where 100 = 1%)
    pub fn basis_points(self, bps: u32) -> Result<Self> {
        let value = self
            .value
            .checked_mul(bps as i128)
            .ok_or(OpeniBankError::AmountOverflow)?
            / 10000;
        Ok(Self { value, ..self })
    }

    // Convenience constructors for IUSD

    /// Create an IUSD amount from human value
    pub fn iusd(value: f64) -> Self {
        Self::from_human(value, Currency::iusd())
    }

    /// Create an IUSD amount from smallest units
    pub fn iusd_wei(value: i128) -> Self {
        Self::new(value, Currency::iusd(), STANDARD_DECIMALS)
    }

    /// Create a zero IUSD amount
    pub fn iusd_zero() -> Self {
        Self::zero(Currency::iusd())
    }
}

impl Default for Amount {
    fn default() -> Self {
        Self::iusd_zero()
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let human = self.to_human();
        // Format with appropriate precision based on currency
        let precision = match self.currency {
            Currency::Fiat(c) => c.decimals() as usize,
            Currency::Crypto(c) => c.decimals().min(8) as usize,
            Currency::Synthetic(_) => 2,
        };
        write!(f, "{:.prec$} {}", human, self.currency, prec = precision)
    }
}

impl PartialOrd for Amount {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.currency != other.currency {
            return None;
        }

        // Scale to same decimals for comparison
        let (a, b) = if self.decimals != other.decimals {
            let max_decimals = self.decimals.max(other.decimals);
            match (self.scale_to(max_decimals), other.scale_to(max_decimals)) {
                (Ok(a), Ok(b)) => (a.value, b.value),
                _ => return None,
            }
        } else {
            (self.value, other.value)
        };

        a.partial_cmp(&b)
    }
}

impl Ord for Amount {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

// Implement Add trait for convenience (panics on error)
impl Add for Amount {
    type Output = Self;

    fn add(self, other: Self) -> Self::Output {
        self.checked_add(other).expect("Amount addition overflow")
    }
}

// Implement Sub trait for convenience (panics on error)
impl Sub for Amount {
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self.checked_sub(other).expect("Amount subtraction underflow")
    }
}

/// Spending limits for permits and budgets
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpendingLimits {
    /// Maximum per single transaction
    pub per_transaction: Option<Amount>,
    /// Maximum per day
    pub daily: Option<Amount>,
    /// Maximum per week
    pub weekly: Option<Amount>,
    /// Maximum per month
    pub monthly: Option<Amount>,
    /// Maximum total (lifetime)
    pub total: Option<Amount>,
    /// Amount already spent against this limit
    pub spent: Amount,
}

impl SpendingLimits {
    /// Create limits with only a daily limit
    pub fn daily(amount: Amount) -> Self {
        Self {
            daily: Some(amount),
            spent: Amount::zero(amount.currency),
            ..Default::default()
        }
    }

    /// Create limits with a per-transaction limit
    pub fn per_transaction(amount: Amount) -> Self {
        Self {
            per_transaction: Some(amount),
            spent: Amount::zero(amount.currency),
            ..Default::default()
        }
    }

    /// Create limits with a total limit
    pub fn total(amount: Amount) -> Self {
        Self {
            total: Some(amount),
            spent: Amount::zero(amount.currency),
            ..Default::default()
        }
    }

    /// Check if an amount can be spent
    pub fn can_spend(&self, amount: &Amount) -> bool {
        // Check per-transaction limit
        if let Some(ref limit) = self.per_transaction {
            if amount > limit {
                return false;
            }
        }

        // Check total limit
        if let Some(ref limit) = self.total {
            if let Ok(new_total) = self.spent.checked_add(*amount) {
                if &new_total > limit {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Record a spend (updates the spent amount)
    pub fn record_spend(&mut self, amount: Amount) -> Result<()> {
        self.spent = self.spent.checked_add(amount)?;
        Ok(())
    }

    /// Get remaining budget (if total limit is set)
    pub fn remaining(&self) -> Option<Amount> {
        self.total
            .as_ref()
            .and_then(|limit| limit.checked_sub(self.spent).ok())
    }
}

impl Default for SpendingLimits {
    fn default() -> Self {
        Self {
            per_transaction: None,
            daily: None,
            weekly: None,
            monthly: None,
            total: None,
            spent: Amount::iusd_zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amount_creation() {
        let amt = Amount::iusd(100.50);
        assert_eq!(amt.to_human(), 100.50);
        assert_eq!(amt.currency, Currency::iusd());
    }

    #[test]
    fn test_amount_arithmetic() {
        let a = Amount::iusd(100.0);
        let b = Amount::iusd(50.0);

        let sum = a.checked_add(b).unwrap();
        assert_eq!(sum.to_human(), 150.0);

        let diff = a.checked_sub(b).unwrap();
        assert_eq!(diff.to_human(), 50.0);
    }

    #[test]
    fn test_amount_currency_mismatch() {
        let iusd = Amount::iusd(100.0);
        let eth = Amount::from_human(1.0, Currency::eth());

        assert!(iusd.checked_add(eth).is_err());
    }

    #[test]
    fn test_amount_comparison() {
        let a = Amount::iusd(100.0);
        let b = Amount::iusd(50.0);
        let c = Amount::iusd(100.0);

        assert!(a > b);
        assert!(b < a);
        assert!(a == c);
    }

    #[test]
    fn test_amount_scaling() {
        let amt = Amount::new(10050, Currency::usd(), 2); // $100.50
        let scaled = amt.scale_to(18).unwrap();
        assert_eq!(scaled.decimals, 18);

        // Value should be multiplied by 10^16
        assert_eq!(scaled.value, 10050 * 10i128.pow(16));
    }

    #[test]
    fn test_spending_limits() {
        let mut limits = SpendingLimits::daily(Amount::iusd(1000.0));

        assert!(limits.can_spend(&Amount::iusd(500.0)));
        limits.record_spend(Amount::iusd(500.0)).unwrap();

        assert!(limits.can_spend(&Amount::iusd(400.0)));
        assert!(!limits.can_spend(&Amount::iusd(600.0)));
    }

    #[test]
    fn test_percentage_and_basis_points() {
        let amt = Amount::iusd(1000.0);

        let ten_percent = amt.percentage(10).unwrap();
        assert_eq!(ten_percent.to_human(), 100.0);

        let fifty_bps = amt.basis_points(50).unwrap(); // 0.5%
        assert_eq!(fifty_bps.to_human(), 5.0);
    }
}
