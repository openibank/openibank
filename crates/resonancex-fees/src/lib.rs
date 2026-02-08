//! ResonanceX Fee Engine
//!
//! Tiered fee calculation engine that beats Binance on pricing.
//! 
//! # Fee Structure
//!
//! | Tier     | 30d Volume    | Maker  | Taker  |
//! |----------|---------------|--------|--------|
//! | Standard | < $50K        | 0.08%  | 0.10%  |
//! | Bronze   | $50K-$500K    | 0.06%  | 0.08%  |
//! | Silver   | $500K-$5M     | 0.04%  | 0.06%  |
//! | Gold     | $5M-$50M      | 0.02%  | 0.04%  |
//! | Diamond  | > $50M        | 0.00%  | 0.02%  |
//!
//! OBK stakers get 25% off all fees.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use openibank_types::AgentId;
use resonancex_types::MarketId;

/// Fee calculation errors
#[derive(Debug, Error)]
pub enum FeeError {
    #[error("Invalid fee rate: {0}")]
    InvalidRate(String),
    
    #[error("User not found: {0}")]
    UserNotFound(AgentId),
}

pub type FeeResult<T> = Result<T, FeeError>;

/// Fee tier levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FeeTier {
    Standard,
    Bronze,
    Silver,
    Gold,
    Diamond,
    VIP,
    MarketMaker,
}

impl FeeTier {
    /// Get the volume threshold for this tier (in USD)
    pub fn volume_threshold(&self) -> Decimal {
        match self {
            FeeTier::Standard => Decimal::ZERO,
            FeeTier::Bronze => dec!(50_000),
            FeeTier::Silver => dec!(500_000),
            FeeTier::Gold => dec!(5_000_000),
            FeeTier::Diamond => dec!(50_000_000),
            FeeTier::VIP => dec!(100_000_000),
            FeeTier::MarketMaker => Decimal::ZERO, // Special status
        }
    }

    /// Get the base maker fee for this tier
    pub fn maker_fee(&self) -> Decimal {
        match self {
            FeeTier::Standard => dec!(0.0008),     // 0.08%
            FeeTier::Bronze => dec!(0.0006),       // 0.06%
            FeeTier::Silver => dec!(0.0004),       // 0.04%
            FeeTier::Gold => dec!(0.0002),         // 0.02%
            FeeTier::Diamond => Decimal::ZERO,     // 0.00%
            FeeTier::VIP => Decimal::ZERO,         // 0.00%
            FeeTier::MarketMaker => dec!(-0.0001), // -0.01% (rebate)
        }
    }

    /// Get the base taker fee for this tier
    pub fn taker_fee(&self) -> Decimal {
        match self {
            FeeTier::Standard => dec!(0.0010),    // 0.10%
            FeeTier::Bronze => dec!(0.0008),      // 0.08%
            FeeTier::Silver => dec!(0.0006),      // 0.06%
            FeeTier::Gold => dec!(0.0004),        // 0.04%
            FeeTier::Diamond => dec!(0.0002),     // 0.02%
            FeeTier::VIP => dec!(0.0001),         // 0.01%
            FeeTier::MarketMaker => dec!(0.0002), // 0.02%
        }
    }

    /// Determine tier from 30-day trading volume
    pub fn from_volume(volume_usd: Decimal) -> Self {
        if volume_usd >= dec!(100_000_000) {
            FeeTier::VIP
        } else if volume_usd >= dec!(50_000_000) {
            FeeTier::Diamond
        } else if volume_usd >= dec!(5_000_000) {
            FeeTier::Gold
        } else if volume_usd >= dec!(500_000) {
            FeeTier::Silver
        } else if volume_usd >= dec!(50_000) {
            FeeTier::Bronze
        } else {
            FeeTier::Standard
        }
    }
}

impl Default for FeeTier {
    fn default() -> Self {
        FeeTier::Standard
    }
}

/// OBK staking discount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OBKDiscount {
    /// Minimum OBK staked for discount
    pub min_staked: Decimal,
    /// Discount percentage (0.25 = 25% off)
    pub discount_rate: Decimal,
}

impl Default for OBKDiscount {
    fn default() -> Self {
        Self {
            min_staked: dec!(100), // 100 OBK minimum
            discount_rate: dec!(0.25), // 25% off
        }
    }
}

/// User fee profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeeProfile {
    /// User ID
    pub user_id: AgentId,
    /// Current fee tier
    pub tier: FeeTier,
    /// 30-day trading volume in USD
    pub volume_30d: Decimal,
    /// OBK tokens staked
    pub obk_staked: Decimal,
    /// Custom fee override (for VIP deals)
    pub custom_maker_fee: Option<Decimal>,
    pub custom_taker_fee: Option<Decimal>,
    /// Is market maker program member
    pub is_market_maker: bool,
    /// Referrer ID (for fee sharing)
    pub referrer: Option<AgentId>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl UserFeeProfile {
    /// Create a new user profile with default tier
    pub fn new(user_id: AgentId) -> Self {
        Self {
            user_id,
            tier: FeeTier::Standard,
            volume_30d: Decimal::ZERO,
            obk_staked: Decimal::ZERO,
            custom_maker_fee: None,
            custom_taker_fee: None,
            is_market_maker: false,
            referrer: None,
            updated_at: Utc::now(),
        }
    }

    /// Update tier based on volume
    pub fn update_tier(&mut self) {
        if self.is_market_maker {
            self.tier = FeeTier::MarketMaker;
        } else {
            self.tier = FeeTier::from_volume(self.volume_30d);
        }
        self.updated_at = Utc::now();
    }
}

/// Calculated fee for a trade
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedFee {
    /// Base fee before discounts
    pub base_fee: Decimal,
    /// OBK discount amount
    pub obk_discount: Decimal,
    /// Final fee after all discounts
    pub final_fee: Decimal,
    /// Fee rate applied
    pub fee_rate: Decimal,
    /// Is this a maker or taker fee
    pub is_maker: bool,
    /// Quote currency
    pub fee_currency: String,
    /// Referral fee share (goes to referrer)
    pub referral_share: Decimal,
    /// Platform fee (revenue)
    pub platform_fee: Decimal,
}

/// Fee engine for calculating trading fees
pub struct FeeEngine {
    /// User fee profiles
    profiles: RwLock<HashMap<AgentId, UserFeeProfile>>,
    /// OBK discount configuration
    obk_discount: OBKDiscount,
    /// Referral fee share rate (20% of fees to referrer)
    referral_share_rate: Decimal,
    /// Market-specific fee overrides
    market_fees: RwLock<HashMap<MarketId, (Decimal, Decimal)>>, // (maker, taker)
}

impl FeeEngine {
    /// Create a new fee engine
    pub fn new() -> Self {
        Self {
            profiles: RwLock::new(HashMap::new()),
            obk_discount: OBKDiscount::default(),
            referral_share_rate: dec!(0.20), // 20% to referrer
            market_fees: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new user
    pub fn register_user(&self, user_id: AgentId) {
        let profile = UserFeeProfile::new(user_id.clone());
        self.profiles.write().insert(user_id, profile);
    }

    /// Get user fee profile
    pub fn get_profile(&self, user_id: &AgentId) -> Option<UserFeeProfile> {
        self.profiles.read().get(user_id).cloned()
    }

    /// Update user's 30-day volume
    pub fn update_volume(&self, user_id: &AgentId, volume_delta: Decimal) -> FeeResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles.get_mut(user_id)
            .ok_or_else(|| FeeError::UserNotFound(user_id.clone()))?;
        
        profile.volume_30d += volume_delta;
        profile.update_tier();
        Ok(())
    }

    /// Update user's OBK stake
    pub fn update_obk_stake(&self, user_id: &AgentId, staked: Decimal) -> FeeResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles.get_mut(user_id)
            .ok_or_else(|| FeeError::UserNotFound(user_id.clone()))?;
        
        profile.obk_staked = staked;
        profile.updated_at = Utc::now();
        Ok(())
    }

    /// Set user as market maker
    pub fn set_market_maker(&self, user_id: &AgentId, is_mm: bool) -> FeeResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles.get_mut(user_id)
            .ok_or_else(|| FeeError::UserNotFound(user_id.clone()))?;
        
        profile.is_market_maker = is_mm;
        profile.update_tier();
        Ok(())
    }

    /// Set referrer for a user
    pub fn set_referrer(&self, user_id: &AgentId, referrer_id: AgentId) -> FeeResult<()> {
        let mut profiles = self.profiles.write();
        let profile = profiles.get_mut(user_id)
            .ok_or_else(|| FeeError::UserNotFound(user_id.clone()))?;
        
        profile.referrer = Some(referrer_id);
        Ok(())
    }

    /// Set market-specific fees
    pub fn set_market_fees(&self, market: MarketId, maker: Decimal, taker: Decimal) {
        self.market_fees.write().insert(market, (maker, taker));
    }

    /// Calculate fee for a trade
    pub fn calculate_fee(
        &self,
        user_id: &AgentId,
        market: &MarketId,
        quote_amount: Decimal,
        is_maker: bool,
    ) -> CalculatedFee {
        // Get user profile or use default
        let profile = self.get_profile(user_id)
            .unwrap_or_else(|| UserFeeProfile::new(user_id.clone()));

        // Determine base fee rate
        let (base_maker, base_taker) = self.market_fees.read()
            .get(market)
            .cloned()
            .unwrap_or_else(|| (profile.tier.maker_fee(), profile.tier.taker_fee()));

        let base_rate = if is_maker {
            profile.custom_maker_fee.unwrap_or(base_maker)
        } else {
            profile.custom_taker_fee.unwrap_or(base_taker)
        };

        // Calculate base fee
        let base_fee = quote_amount * base_rate;

        // Apply OBK discount if eligible
        let obk_discount = if profile.obk_staked >= self.obk_discount.min_staked {
            base_fee * self.obk_discount.discount_rate
        } else {
            Decimal::ZERO
        };

        let fee_after_obk = base_fee - obk_discount;

        // Calculate referral share (only if fee is positive)
        let referral_share = if profile.referrer.is_some() && fee_after_obk > Decimal::ZERO {
            fee_after_obk * self.referral_share_rate
        } else {
            Decimal::ZERO
        };

        let platform_fee = fee_after_obk - referral_share;

        CalculatedFee {
            base_fee,
            obk_discount,
            final_fee: fee_after_obk,
            fee_rate: base_rate,
            is_maker,
            fee_currency: "IUSD".to_string(),
            referral_share,
            platform_fee,
        }
    }

    /// Get tier info for display
    pub fn get_tier_info(&self, user_id: &AgentId) -> TierInfo {
        let profile = self.get_profile(user_id);
        
        match profile {
            Some(p) => {
                let next_tier = match p.tier {
                    FeeTier::Standard => Some(FeeTier::Bronze),
                    FeeTier::Bronze => Some(FeeTier::Silver),
                    FeeTier::Silver => Some(FeeTier::Gold),
                    FeeTier::Gold => Some(FeeTier::Diamond),
                    FeeTier::Diamond => Some(FeeTier::VIP),
                    FeeTier::VIP | FeeTier::MarketMaker => None,
                };

                let volume_to_next = next_tier.map(|t| t.volume_threshold() - p.volume_30d);

                TierInfo {
                    current_tier: p.tier,
                    volume_30d: p.volume_30d,
                    maker_fee: p.custom_maker_fee.unwrap_or(p.tier.maker_fee()),
                    taker_fee: p.custom_taker_fee.unwrap_or(p.tier.taker_fee()),
                    obk_staked: p.obk_staked,
                    obk_discount_active: p.obk_staked >= self.obk_discount.min_staked,
                    next_tier,
                    volume_to_next,
                }
            }
            None => TierInfo::default(),
        }
    }
}

impl Default for FeeEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Tier information for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierInfo {
    pub current_tier: FeeTier,
    pub volume_30d: Decimal,
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub obk_staked: Decimal,
    pub obk_discount_active: bool,
    pub next_tier: Option<FeeTier>,
    pub volume_to_next: Option<Decimal>,
}

impl Default for TierInfo {
    fn default() -> Self {
        Self {
            current_tier: FeeTier::Standard,
            volume_30d: Decimal::ZERO,
            maker_fee: FeeTier::Standard.maker_fee(),
            taker_fee: FeeTier::Standard.taker_fee(),
            obk_staked: Decimal::ZERO,
            obk_discount_active: false,
            next_tier: Some(FeeTier::Bronze),
            volume_to_next: Some(dec!(50_000)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_from_volume() {
        assert_eq!(FeeTier::from_volume(dec!(0)), FeeTier::Standard);
        assert_eq!(FeeTier::from_volume(dec!(49_999)), FeeTier::Standard);
        assert_eq!(FeeTier::from_volume(dec!(50_000)), FeeTier::Bronze);
        assert_eq!(FeeTier::from_volume(dec!(500_000)), FeeTier::Silver);
        assert_eq!(FeeTier::from_volume(dec!(5_000_000)), FeeTier::Gold);
        assert_eq!(FeeTier::from_volume(dec!(50_000_000)), FeeTier::Diamond);
        assert_eq!(FeeTier::from_volume(dec!(100_000_000)), FeeTier::VIP);
    }

    #[test]
    fn test_fee_rates() {
        // Standard tier should be 0.08% maker, 0.10% taker
        assert_eq!(FeeTier::Standard.maker_fee(), dec!(0.0008));
        assert_eq!(FeeTier::Standard.taker_fee(), dec!(0.0010));

        // Diamond should be 0% maker, 0.02% taker
        assert_eq!(FeeTier::Diamond.maker_fee(), Decimal::ZERO);
        assert_eq!(FeeTier::Diamond.taker_fee(), dec!(0.0002));

        // Market maker should have rebate
        assert!(FeeTier::MarketMaker.maker_fee() < Decimal::ZERO);
    }

    #[test]
    fn test_fee_calculation() {
        let engine = FeeEngine::new();
        let user_id = AgentId::new();
        engine.register_user(user_id.clone());

        let market = MarketId::new("ETH_IUSD");
        let quote_amount = dec!(1000); // $1000 trade

        // Standard tier taker fee: 0.10% of $1000 = $1
        let fee = engine.calculate_fee(&user_id, &market, quote_amount, false);
        assert_eq!(fee.base_fee, dec!(1.0));
        assert_eq!(fee.final_fee, dec!(1.0));
        assert!(!fee.is_maker);
    }

    #[test]
    fn test_obk_discount() {
        let engine = FeeEngine::new();
        let user_id = AgentId::new();
        engine.register_user(user_id.clone());
        
        // Stake 100 OBK to get discount
        engine.update_obk_stake(&user_id, dec!(100)).unwrap();

        let market = MarketId::new("ETH_IUSD");
        let quote_amount = dec!(1000);

        let fee = engine.calculate_fee(&user_id, &market, quote_amount, false);
        
        // Base fee $1, with 25% discount = $0.75
        assert_eq!(fee.base_fee, dec!(1.0));
        assert_eq!(fee.obk_discount, dec!(0.25));
        assert_eq!(fee.final_fee, dec!(0.75));
    }

    #[test]
    fn test_volume_tier_upgrade() {
        let engine = FeeEngine::new();
        let user_id = AgentId::new();
        engine.register_user(user_id.clone());

        // Start at Standard
        let profile = engine.get_profile(&user_id).unwrap();
        assert_eq!(profile.tier, FeeTier::Standard);

        // Add $50K volume -> Bronze
        engine.update_volume(&user_id, dec!(50_000)).unwrap();
        let profile = engine.get_profile(&user_id).unwrap();
        assert_eq!(profile.tier, FeeTier::Bronze);

        // Add $450K more -> Silver
        engine.update_volume(&user_id, dec!(450_000)).unwrap();
        let profile = engine.get_profile(&user_id).unwrap();
        assert_eq!(profile.tier, FeeTier::Silver);
    }

    #[test]
    fn test_referral_share() {
        let engine = FeeEngine::new();
        let user_id = AgentId::new();
        let referrer_id = AgentId::new();
        
        engine.register_user(user_id.clone());
        engine.register_user(referrer_id.clone());
        engine.set_referrer(&user_id, referrer_id).unwrap();

        let market = MarketId::new("ETH_IUSD");
        let quote_amount = dec!(1000);

        let fee = engine.calculate_fee(&user_id, &market, quote_amount, false);
        
        // $1 fee, 20% to referrer = $0.20
        assert_eq!(fee.final_fee, dec!(1.0));
        assert_eq!(fee.referral_share, dec!(0.20));
        assert_eq!(fee.platform_fee, dec!(0.80));
    }

    #[test]
    fn test_market_maker_rebate() {
        let engine = FeeEngine::new();
        let user_id = AgentId::new();
        engine.register_user(user_id.clone());
        engine.set_market_maker(&user_id, true).unwrap();

        let profile = engine.get_profile(&user_id).unwrap();
        assert_eq!(profile.tier, FeeTier::MarketMaker);

        let market = MarketId::new("ETH_IUSD");
        let quote_amount = dec!(1000);

        // Maker fee should be negative (rebate)
        let fee = engine.calculate_fee(&user_id, &market, quote_amount, true);
        assert!(fee.base_fee < Decimal::ZERO);
    }
}
