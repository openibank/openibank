//! Currency types for OpeniBank
//!
//! Supports fiat, crypto, and synthetic (OpeniBank-issued) currencies.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Fiat currency codes (ISO 4217)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiatCurrency {
    USD,
    EUR,
    GBP,
    JPY,
    CNY,
    CHF,
    AUD,
    CAD,
    HKD,
    SGD,
    KRW,
    INR,
    BRL,
    MXN,
    ZAR,
}

impl FiatCurrency {
    /// Get the standard decimal places for this currency
    pub fn decimals(&self) -> u8 {
        match self {
            Self::JPY | Self::KRW => 0,
            _ => 2,
        }
    }

    /// Get the ISO 4217 code
    pub fn code(&self) -> &'static str {
        match self {
            Self::USD => "USD",
            Self::EUR => "EUR",
            Self::GBP => "GBP",
            Self::JPY => "JPY",
            Self::CNY => "CNY",
            Self::CHF => "CHF",
            Self::AUD => "AUD",
            Self::CAD => "CAD",
            Self::HKD => "HKD",
            Self::SGD => "SGD",
            Self::KRW => "KRW",
            Self::INR => "INR",
            Self::BRL => "BRL",
            Self::MXN => "MXN",
            Self::ZAR => "ZAR",
        }
    }
}

impl fmt::Display for FiatCurrency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

/// Cryptocurrency types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CryptoCurrency {
    /// Bitcoin
    BTC,
    /// Ethereum
    ETH,
    /// USD Coin
    USDC,
    /// Tether
    USDT,
    /// Solana
    SOL,
    /// Polygon/Matic
    MATIC,
    /// Avalanche
    AVAX,
    /// Arbitrum
    ARB,
    /// Optimism
    OP,
    /// Base
    BASE,
    /// Wrapped Ether
    WETH,
    /// DAI stablecoin
    DAI,
}

impl CryptoCurrency {
    /// Get the standard decimal places for this currency
    pub fn decimals(&self) -> u8 {
        match self {
            Self::BTC => 8,
            Self::USDC | Self::USDT => 6,
            _ => 18,
        }
    }

    /// Get the symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::BTC => "BTC",
            Self::ETH => "ETH",
            Self::USDC => "USDC",
            Self::USDT => "USDT",
            Self::SOL => "SOL",
            Self::MATIC => "MATIC",
            Self::AVAX => "AVAX",
            Self::ARB => "ARB",
            Self::OP => "OP",
            Self::BASE => "BASE",
            Self::WETH => "WETH",
            Self::DAI => "DAI",
        }
    }
}

impl fmt::Display for CryptoCurrency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// OpeniBank-issued synthetic currencies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SyntheticCurrency {
    /// OpeniBank USD - the primary stablecoin
    IUSD,
    /// OpeniBank EUR
    IEUR,
    /// OpeniBank GBP
    IGBP,
    /// OpeniBank JPY
    IJPY,
    /// OpeniBank CHF
    ICHF,
}

impl SyntheticCurrency {
    /// Get the standard decimal places (always 18 for synthetics)
    pub fn decimals(&self) -> u8 {
        18
    }

    /// Get the symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::IUSD => "IUSD",
            Self::IEUR => "IEUR",
            Self::IGBP => "IGBP",
            Self::IJPY => "IJPY",
            Self::ICHF => "ICHF",
        }
    }

    /// Get the backing fiat currency
    pub fn backing_fiat(&self) -> FiatCurrency {
        match self {
            Self::IUSD => FiatCurrency::USD,
            Self::IEUR => FiatCurrency::EUR,
            Self::IGBP => FiatCurrency::GBP,
            Self::IJPY => FiatCurrency::JPY,
            Self::ICHF => FiatCurrency::CHF,
        }
    }
}

impl fmt::Display for SyntheticCurrency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Unified currency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    /// Fiat currency
    Fiat(FiatCurrency),
    /// Cryptocurrency
    Crypto(CryptoCurrency),
    /// OpeniBank synthetic currency
    Synthetic(SyntheticCurrency),
}

impl Currency {
    /// Get the standard decimal places for this currency
    pub fn decimals(&self) -> u8 {
        match self {
            Self::Fiat(c) => c.decimals(),
            Self::Crypto(c) => c.decimals(),
            Self::Synthetic(c) => c.decimals(),
        }
    }

    /// Get the currency symbol/code
    pub fn symbol(&self) -> String {
        match self {
            Self::Fiat(c) => c.code().to_string(),
            Self::Crypto(c) => c.symbol().to_string(),
            Self::Synthetic(c) => c.symbol().to_string(),
        }
    }

    /// Check if this is a stablecoin (pegged to fiat)
    pub fn is_stablecoin(&self) -> bool {
        matches!(
            self,
            Self::Synthetic(_) | Self::Crypto(CryptoCurrency::USDC | CryptoCurrency::USDT | CryptoCurrency::DAI)
        )
    }

    /// Check if this is a fiat currency
    pub fn is_fiat(&self) -> bool {
        matches!(self, Self::Fiat(_))
    }

    /// Check if this is a cryptocurrency
    pub fn is_crypto(&self) -> bool {
        matches!(self, Self::Crypto(_))
    }

    /// Check if this is an OpeniBank synthetic
    pub fn is_synthetic(&self) -> bool {
        matches!(self, Self::Synthetic(_))
    }

    // Convenience constructors
    pub fn usd() -> Self {
        Self::Fiat(FiatCurrency::USD)
    }

    pub fn eur() -> Self {
        Self::Fiat(FiatCurrency::EUR)
    }

    pub fn eth() -> Self {
        Self::Crypto(CryptoCurrency::ETH)
    }

    pub fn btc() -> Self {
        Self::Crypto(CryptoCurrency::BTC)
    }

    pub fn usdc() -> Self {
        Self::Crypto(CryptoCurrency::USDC)
    }

    pub fn iusd() -> Self {
        Self::Synthetic(SyntheticCurrency::IUSD)
    }
}

impl Default for Currency {
    fn default() -> Self {
        Self::iusd()
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.symbol())
    }
}

/// Blockchain/chain identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    /// Ethereum mainnet
    Ethereum,
    /// Polygon
    Polygon,
    /// Arbitrum One
    Arbitrum,
    /// Optimism
    Optimism,
    /// Base
    Base,
    /// Avalanche C-Chain
    Avalanche,
    /// Solana
    Solana,
    /// Bitcoin
    Bitcoin,
    /// Custom chain by ID
    Custom { chain_id: u64, name: String },
}

impl Chain {
    /// Get the chain ID (for EVM chains)
    pub fn chain_id(&self) -> Option<u64> {
        match self {
            Self::Ethereum => Some(1),
            Self::Polygon => Some(137),
            Self::Arbitrum => Some(42161),
            Self::Optimism => Some(10),
            Self::Base => Some(8453),
            Self::Avalanche => Some(43114),
            Self::Custom { chain_id, .. } => Some(*chain_id),
            Self::Solana | Self::Bitcoin => None,
        }
    }

    /// Check if this is an EVM-compatible chain
    pub fn is_evm(&self) -> bool {
        !matches!(self, Self::Solana | Self::Bitcoin)
    }
}

impl fmt::Display for Chain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ethereum => write!(f, "Ethereum"),
            Self::Polygon => write!(f, "Polygon"),
            Self::Arbitrum => write!(f, "Arbitrum"),
            Self::Optimism => write!(f, "Optimism"),
            Self::Base => write!(f, "Base"),
            Self::Avalanche => write!(f, "Avalanche"),
            Self::Solana => write!(f, "Solana"),
            Self::Bitcoin => write!(f, "Bitcoin"),
            Self::Custom { name, .. } => write!(f, "{}", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_decimals() {
        assert_eq!(Currency::usd().decimals(), 2);
        assert_eq!(Currency::eth().decimals(), 18);
        assert_eq!(Currency::usdc().decimals(), 6);
        assert_eq!(Currency::iusd().decimals(), 18);
    }

    #[test]
    fn test_stablecoin_detection() {
        assert!(Currency::iusd().is_stablecoin());
        assert!(Currency::usdc().is_stablecoin());
        assert!(!Currency::eth().is_stablecoin());
        assert!(!Currency::usd().is_stablecoin());
    }

    #[test]
    fn test_chain_ids() {
        assert_eq!(Chain::Ethereum.chain_id(), Some(1));
        assert_eq!(Chain::Base.chain_id(), Some(8453));
        assert_eq!(Chain::Bitcoin.chain_id(), None);
    }
}
