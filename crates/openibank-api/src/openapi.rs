//! OpenAPI Documentation
//!
//! Auto-generated OpenAPI 3.0 specification for the OpeniBank API.

use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme, HttpBuilder, HttpAuthScheme};

use crate::dto;
use crate::handlers;
use crate::error::ErrorResponse;

/// OpeniBank API Documentation
#[derive(OpenApi)]
#[openapi(
    info(
        title = "OpeniBank API",
        description = "Production-grade REST API for OpeniBank trading platform. Binance-compatible endpoints for seamless integration.",
        version = "1.0.0",
        contact(
            name = "OpeniBank Team",
            email = "api@openibank.io"
        ),
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        )
    ),
    servers(
        (url = "https://api.openibank.io", description = "Production"),
        (url = "https://testnet-api.openibank.io", description = "Testnet"),
        (url = "http://localhost:3000", description = "Local Development")
    ),
    paths(
        // Health
        handlers::health::health_check,
        handlers::health::readiness_check,
        handlers::health::server_time,
        handlers::health::ping,
        // Auth
        handlers::auth::login,
        handlers::auth::register,
        handlers::auth::refresh_token,
        handlers::auth::logout,
        handlers::auth::setup_two_factor,
        handlers::auth::verify_two_factor,
        handlers::auth::disable_two_factor,
        handlers::auth::create_api_key,
        handlers::auth::list_api_keys,
        handlers::auth::delete_api_key,
        // Account
        handlers::account::get_account_info,
        handlers::account::get_account_status,
        handlers::account::get_trade_fees,
        handlers::account::get_balances,
        // Wallet
        handlers::wallet::get_deposit_address,
        handlers::wallet::get_deposit_history,
        handlers::wallet::submit_withdrawal,
        handlers::wallet::get_withdrawal_history,
        handlers::wallet::get_all_coins_info,
        handlers::wallet::internal_transfer,
        // Order
        handlers::order::create_order,
        handlers::order::query_order,
        handlers::order::cancel_order,
        handlers::order::get_open_orders,
        handlers::order::get_all_orders,
        handlers::order::cancel_all_orders,
        handlers::order::get_account_trades,
        // Market
        handlers::market::get_exchange_info,
        handlers::market::get_order_book,
        handlers::market::get_recent_trades,
        handlers::market::get_agg_trades,
        handlers::market::get_klines,
        handlers::market::get_ticker_24hr,
        handlers::market::get_price_ticker,
        handlers::market::get_book_ticker,
        handlers::market::get_avg_price,
    ),
    components(
        schemas(
            // Common
            ErrorResponse,
            dto::ServerTimeResponse,
            dto::PaginationParams,
            // Auth
            dto::LoginRequest,
            dto::LoginResponse,
            dto::RegisterRequest,
            dto::RegisterResponse,
            dto::RefreshTokenRequest,
            dto::RefreshTokenResponse,
            dto::LogoutRequest,
            dto::LogoutResponse,
            dto::TwoFactorSetupResponse,
            dto::TwoFactorVerifyRequest,
            dto::TwoFactorVerifyResponse,
            dto::CreateApiKeyRequest,
            dto::CreateApiKeyResponse,
            dto::ApiKeyInfo,
            dto::DeleteApiKeyRequest,
            // Account
            dto::AccountInfo,
            dto::BalanceInfo,
            dto::CommissionRates,
            dto::TradeFeeInfo,
            dto::AccountStatus,
            dto::NotificationSettings,
            // Wallet
            dto::DepositAddressResponse,
            dto::DepositRecord,
            dto::WithdrawalRequest,
            dto::WithdrawalResponse,
            dto::WithdrawalRecord,
            dto::AssetInfo,
            dto::NetworkInfo,
            dto::InternalTransferRequest,
            dto::InternalTransferResponse,
            // Order
            dto::CreateOrderRequest,
            dto::OrderInfo,
            dto::CancelOrderRequest,
            dto::CancelOrderResponse,
            dto::AccountTrade,
            dto::OrderSide,
            dto::OrderType,
            dto::TimeInForce,
            dto::OrderStatus,
            // Market
            dto::ExchangeInfo,
            dto::SymbolInfo,
            dto::SymbolFilter,
            dto::RateLimit,
            dto::OrderBook,
            dto::TradeRecord,
            dto::AggTrade,
            dto::AvgPrice,
        )
    ),
    tags(
        (name = "Health", description = "Service health and status"),
        (name = "General", description = "General endpoints (ping, time)"),
        (name = "Authentication", description = "User authentication and session management"),
        (name = "API Keys", description = "API key management"),
        (name = "Account", description = "Account information and settings"),
        (name = "Wallet", description = "Deposit and withdrawal operations"),
        (name = "Trading", description = "Order placement and management"),
        (name = "Market Data", description = "Market data and exchange information")
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

/// Security scheme modifier
pub struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = &mut openapi.components {
            components.add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build()
                )
            );
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("X-MBX-APIKEY")))
            );
        }
    }
}

/// Get the OpenAPI JSON specification
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_json().expect("Failed to serialize OpenAPI spec")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let spec = ApiDoc::openapi();
        assert_eq!(spec.info.title, "OpeniBank API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[test]
    fn test_openapi_json() {
        let json = openapi_json();
        assert!(json.contains("OpeniBank API"));
    }
}
