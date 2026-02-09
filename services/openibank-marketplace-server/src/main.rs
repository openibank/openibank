//! OpenIBank Marketplace Server
//!
//! A production-ready agent service marketplace for financial services.
//! Provides service discovery, agent profiles, reviews, and integration guides.

use axum::{
    extract::{Path, Query, State},
    http::{Method, StatusCode},
    response::{Html, Json},
    routing::get,
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

// ============================================================================
// Data Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCategory {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,
    pub service_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingTier {
    pub name: String,
    pub price: f64,
    pub billing_period: String,
    pub features: Vec<String>,
    pub api_calls: Option<u64>,
    pub support_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentService {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub short_description: String,
    pub long_description: String,
    pub category_id: String,
    pub agent_id: String,
    pub icon_url: String,
    pub banner_url: Option<String>,
    pub website_url: Option<String>,
    pub documentation_url: Option<String>,
    pub pricing_tiers: Vec<PricingTier>,
    pub features: Vec<String>,
    pub tags: Vec<String>,
    pub integration_type: String,
    pub supported_protocols: Vec<String>,
    pub avg_rating: f32,
    pub total_reviews: u32,
    pub total_installs: u64,
    pub is_verified: bool,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub logo_url: String,
    pub website_url: Option<String>,
    pub support_email: Option<String>,
    pub is_verified: bool,
    pub verification_badge: Option<String>,
    pub total_services: u32,
    pub total_installs: u64,
    pub avg_rating: f32,
    pub joined_at: DateTime<Utc>,
    pub social_links: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub service_id: String,
    pub agent_id: String,
    pub user_id: String,
    pub user_name: String,
    pub user_avatar: Option<String>,
    pub rating: u8,
    pub title: String,
    pub content: String,
    pub helpful_count: u32,
    pub created_at: DateTime<Utc>,
    pub verified_purchase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub service_id: String,
    pub daily_requests: Vec<u64>,
    pub monthly_requests: u64,
    pub avg_latency_ms: f32,
    pub uptime_percentage: f32,
    pub error_rate: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationGuide {
    pub service_id: String,
    pub title: String,
    pub content: String,
    pub code_samples: HashMap<String, String>,
    pub prerequisites: Vec<String>,
    pub estimated_time: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    pub category: Option<String>,
    pub min_rating: Option<f32>,
    pub max_price: Option<f64>,
    pub verified_only: Option<bool>,
    pub sort_by: Option<String>,
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceRequest {
    pub name: String,
    pub short_description: String,
    pub long_description: String,
    pub category_id: String,
    pub icon_url: String,
    pub pricing_tiers: Vec<PricingTier>,
    pub features: Vec<String>,
    pub tags: Vec<String>,
    pub integration_type: String,
    pub supported_protocols: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReviewRequest {
    pub rating: u8,
    pub title: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub meta: Option<ResponseMeta>,
}

#[derive(Debug, Serialize)]
pub struct ResponseMeta {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct MarketplaceStats {
    pub total_services: u64,
    pub total_agents: u64,
    pub total_installs: u64,
    pub categories: Vec<ServiceCategory>,
}

// ============================================================================
// Application State
// ============================================================================

pub struct AppState {
    pub services: RwLock<HashMap<String, AgentService>>,
    pub agents: RwLock<HashMap<String, Agent>>,
    pub reviews: RwLock<HashMap<String, Vec<Review>>>,
    pub categories: RwLock<Vec<ServiceCategory>>,
}

impl AppState {
    fn new() -> Self {
        let state = Self {
            services: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            reviews: RwLock::new(HashMap::new()),
            categories: RwLock::new(Vec::new()),
        };
        state
    }
}

// ============================================================================
// Initialize Sample Data
// ============================================================================

async fn init_sample_data(state: &Arc<AppState>) {
    // Initialize categories
    let categories = vec![
        ServiceCategory {
            id: "trading-bots".to_string(),
            name: "Trading Bots".to_string(),
            description: "Automated trading strategies and algorithmic execution".to_string(),
            icon: "chart-line".to_string(),
            service_count: 12,
        },
        ServiceCategory {
            id: "data-analysis".to_string(),
            name: "Data Analysis".to_string(),
            description: "Financial data processing, analytics, and insights".to_string(),
            icon: "chart-bar".to_string(),
            service_count: 18,
        },
        ServiceCategory {
            id: "risk-management".to_string(),
            name: "Risk Management".to_string(),
            description: "Risk assessment, compliance, and monitoring tools".to_string(),
            icon: "shield-check".to_string(),
            service_count: 8,
        },
        ServiceCategory {
            id: "payment-processing".to_string(),
            name: "Payment Processing".to_string(),
            description: "Payment gateways, transfers, and reconciliation".to_string(),
            icon: "credit-card".to_string(),
            service_count: 15,
        },
        ServiceCategory {
            id: "fraud-detection".to_string(),
            name: "Fraud Detection".to_string(),
            description: "Real-time fraud prevention and anomaly detection".to_string(),
            icon: "exclamation-triangle".to_string(),
            service_count: 6,
        },
        ServiceCategory {
            id: "kyc-aml".to_string(),
            name: "KYC/AML".to_string(),
            description: "Identity verification and anti-money laundering".to_string(),
            icon: "user-check".to_string(),
            service_count: 10,
        },
        ServiceCategory {
            id: "market-data".to_string(),
            name: "Market Data".to_string(),
            description: "Real-time and historical market data feeds".to_string(),
            icon: "database".to_string(),
            service_count: 14,
        },
        ServiceCategory {
            id: "portfolio-management".to_string(),
            name: "Portfolio Management".to_string(),
            description: "Portfolio optimization and asset allocation".to_string(),
            icon: "briefcase".to_string(),
            service_count: 9,
        },
    ];
    *state.categories.write().await = categories;

    // Initialize agents
    let mut agents = HashMap::new();

    let agent1 = Agent {
        id: "agent-001".to_string(),
        name: "QuantAlpha Labs".to_string(),
        slug: "quantalpha-labs".to_string(),
        description: "Leading provider of quantitative trading solutions and algorithmic strategies for institutional investors.".to_string(),
        logo_url: "https://api.dicebear.com/7.x/shapes/svg?seed=quantalpha".to_string(),
        website_url: Some("https://quantalpha.io".to_string()),
        support_email: Some("support@quantalpha.io".to_string()),
        is_verified: true,
        verification_badge: Some("Enterprise".to_string()),
        total_services: 4,
        total_installs: 15420,
        avg_rating: 4.8,
        joined_at: Utc::now() - chrono::Duration::days(365),
        social_links: HashMap::from([
            ("twitter".to_string(), "https://twitter.com/quantalpha".to_string()),
            ("github".to_string(), "https://github.com/quantalpha".to_string()),
        ]),
    };

    let agent2 = Agent {
        id: "agent-002".to_string(),
        name: "SecureFinance AI".to_string(),
        slug: "securefinance-ai".to_string(),
        description: "AI-powered risk management and fraud detection for modern financial institutions.".to_string(),
        logo_url: "https://api.dicebear.com/7.x/shapes/svg?seed=securefinance".to_string(),
        website_url: Some("https://securefinance.ai".to_string()),
        support_email: Some("hello@securefinance.ai".to_string()),
        is_verified: true,
        verification_badge: Some("Security Certified".to_string()),
        total_services: 3,
        total_installs: 8750,
        avg_rating: 4.9,
        joined_at: Utc::now() - chrono::Duration::days(280),
        social_links: HashMap::from([
            ("linkedin".to_string(), "https://linkedin.com/company/securefinance".to_string()),
        ]),
    };

    let agent3 = Agent {
        id: "agent-003".to_string(),
        name: "DataStream Pro".to_string(),
        slug: "datastream-pro".to_string(),
        description: "Enterprise-grade market data infrastructure and real-time analytics platform.".to_string(),
        logo_url: "https://api.dicebear.com/7.x/shapes/svg?seed=datastream".to_string(),
        website_url: Some("https://datastreampro.com".to_string()),
        support_email: Some("support@datastreampro.com".to_string()),
        is_verified: true,
        verification_badge: Some("Premier Partner".to_string()),
        total_services: 5,
        total_installs: 22100,
        avg_rating: 4.7,
        joined_at: Utc::now() - chrono::Duration::days(500),
        social_links: HashMap::new(),
    };

    agents.insert("agent-001".to_string(), agent1);
    agents.insert("agent-002".to_string(), agent2);
    agents.insert("agent-003".to_string(), agent3);
    *state.agents.write().await = agents;

    // Initialize services
    let mut services = HashMap::new();

    let service1 = AgentService {
        id: "svc-001".to_string(),
        name: "AlphaTrader Pro".to_string(),
        slug: "alphatrader-pro".to_string(),
        short_description: "AI-powered algorithmic trading with adaptive strategies".to_string(),
        long_description: r#"AlphaTrader Pro is an enterprise-grade algorithmic trading platform that leverages advanced machine learning to identify and execute profitable trading opportunities across multiple asset classes.

## Key Features

- **Adaptive Strategies**: Self-learning algorithms that adjust to changing market conditions
- **Multi-Asset Support**: Trade stocks, forex, crypto, and derivatives from a single platform
- **Risk Controls**: Built-in position sizing, stop-loss, and portfolio limits
- **Real-time Analytics**: Live P&L tracking, performance attribution, and risk metrics
- **API-First Design**: RESTful and WebSocket APIs for seamless integration

## Performance

Our strategies have consistently outperformed benchmarks with a Sharpe ratio of 2.1 and maximum drawdown of 8% over the past 3 years.

## Integration

Connect via our REST API or use pre-built connectors for major brokerages including Interactive Brokers, Alpaca, and TD Ameritrade."#.to_string(),
        category_id: "trading-bots".to_string(),
        agent_id: "agent-001".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=alphatrader".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1611974789855-9c2a0a7236a3?w=1200".to_string()),
        website_url: Some("https://quantalpha.io/alphatrader".to_string()),
        documentation_url: Some("https://docs.quantalpha.io/alphatrader".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Starter".to_string(),
                price: 99.0,
                billing_period: "month".to_string(),
                features: vec![
                    "5 concurrent strategies".to_string(),
                    "Paper trading".to_string(),
                    "Basic analytics".to_string(),
                    "Email support".to_string(),
                ],
                api_calls: Some(10000),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "Professional".to_string(),
                price: 299.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Unlimited strategies".to_string(),
                    "Live trading".to_string(),
                    "Advanced analytics".to_string(),
                    "Priority support".to_string(),
                    "Custom indicators".to_string(),
                ],
                api_calls: Some(100000),
                support_level: "Priority".to_string(),
            },
            PricingTier {
                name: "Enterprise".to_string(),
                price: 999.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Everything in Pro".to_string(),
                    "Dedicated infrastructure".to_string(),
                    "Custom strategy development".to_string(),
                    "24/7 phone support".to_string(),
                    "SLA guarantee".to_string(),
                ],
                api_calls: None,
                support_level: "Dedicated".to_string(),
            },
        ],
        features: vec![
            "Machine Learning".to_string(),
            "Multi-Asset".to_string(),
            "Real-time Execution".to_string(),
            "Risk Management".to_string(),
            "Backtesting".to_string(),
        ],
        tags: vec!["trading".to_string(), "algorithmic".to_string(), "ai".to_string(), "quantitative".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string(), "WebSocket".to_string(), "FIX".to_string()],
        avg_rating: 4.8,
        total_reviews: 127,
        total_installs: 5420,
        is_verified: true,
        is_featured: true,
        created_at: Utc::now() - chrono::Duration::days(180),
        updated_at: Utc::now() - chrono::Duration::days(5),
        status: "active".to_string(),
    };

    let service2 = AgentService {
        id: "svc-002".to_string(),
        name: "FraudShield AI".to_string(),
        slug: "fraudshield-ai".to_string(),
        short_description: "Real-time fraud detection powered by deep learning".to_string(),
        long_description: r#"FraudShield AI provides industry-leading fraud detection capabilities using advanced neural networks and behavioral analytics.

## How It Works

Our multi-layer detection system analyzes transactions in real-time, combining:
- **Device fingerprinting** and geolocation analysis
- **Behavioral biometrics** to identify suspicious patterns
- **Network analysis** to detect organized fraud rings
- **Anomaly detection** for unusual transaction patterns

## Results

- 99.7% fraud detection rate
- <50ms average response time
- 0.01% false positive rate
- $2.3B+ in fraud prevented for our clients

## Integration

Simple REST API with SDKs for Python, Node.js, Java, and Go. Integrate in minutes with our drop-in widgets or build custom flows with our API."#.to_string(),
        category_id: "fraud-detection".to_string(),
        agent_id: "agent-002".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=fraudshield".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1563986768609-322da13575f3?w=1200".to_string()),
        website_url: Some("https://securefinance.ai/fraudshield".to_string()),
        documentation_url: Some("https://docs.securefinance.ai/fraudshield".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Growth".to_string(),
                price: 199.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Up to 50K transactions/month".to_string(),
                    "Basic fraud rules".to_string(),
                    "Dashboard access".to_string(),
                    "Email alerts".to_string(),
                ],
                api_calls: Some(50000),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "Scale".to_string(),
                price: 599.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Up to 500K transactions/month".to_string(),
                    "Advanced ML models".to_string(),
                    "Custom rules engine".to_string(),
                    "Webhook integrations".to_string(),
                    "Priority support".to_string(),
                ],
                api_calls: Some(500000),
                support_level: "Priority".to_string(),
            },
            PricingTier {
                name: "Enterprise".to_string(),
                price: 1999.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Unlimited transactions".to_string(),
                    "Custom model training".to_string(),
                    "On-premise deployment".to_string(),
                    "Dedicated success manager".to_string(),
                    "99.99% SLA".to_string(),
                ],
                api_calls: None,
                support_level: "Dedicated".to_string(),
            },
        ],
        features: vec![
            "Deep Learning".to_string(),
            "Real-time".to_string(),
            "Behavioral Analysis".to_string(),
            "Network Detection".to_string(),
            "Low Latency".to_string(),
        ],
        tags: vec!["fraud".to_string(), "security".to_string(), "ai".to_string(), "compliance".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string(), "GraphQL".to_string()],
        avg_rating: 4.9,
        total_reviews: 89,
        total_installs: 3250,
        is_verified: true,
        is_featured: true,
        created_at: Utc::now() - chrono::Duration::days(150),
        updated_at: Utc::now() - chrono::Duration::days(2),
        status: "active".to_string(),
    };

    let service3 = AgentService {
        id: "svc-003".to_string(),
        name: "MarketPulse".to_string(),
        slug: "marketpulse".to_string(),
        short_description: "Ultra-low latency market data for trading applications".to_string(),
        long_description: r#"MarketPulse delivers institutional-grade market data with microsecond latency and 99.999% uptime.

## Data Coverage

- **Equities**: NYSE, NASDAQ, LSE, TSE, and 50+ global exchanges
- **Forex**: 180+ currency pairs with tick-by-tick updates
- **Crypto**: 500+ cryptocurrencies from major exchanges
- **Derivatives**: Options, futures, and structured products

## Performance

- Sub-millisecond data delivery
- Redundant data centers worldwide
- Historical data going back 20+ years
- Normalized data format across all asset classes

## Use Cases

- High-frequency trading
- Quantitative research
- Risk management systems
- Trading dashboards
- Regulatory reporting"#.to_string(),
        category_id: "market-data".to_string(),
        agent_id: "agent-003".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=marketpulse".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1590283603385-17ffb3a7f29f?w=1200".to_string()),
        website_url: Some("https://datastreampro.com/marketpulse".to_string()),
        documentation_url: Some("https://docs.datastreampro.com/marketpulse".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Developer".to_string(),
                price: 49.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Delayed data (15 min)".to_string(),
                    "10 symbols".to_string(),
                    "REST API".to_string(),
                    "Basic support".to_string(),
                ],
                api_calls: Some(10000),
                support_level: "Community".to_string(),
            },
            PricingTier {
                name: "Trader".to_string(),
                price: 199.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Real-time data".to_string(),
                    "100 symbols".to_string(),
                    "WebSocket streaming".to_string(),
                    "Historical data (5 years)".to_string(),
                    "Email support".to_string(),
                ],
                api_calls: Some(100000),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "Institutional".to_string(),
                price: 999.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Ultra-low latency feed".to_string(),
                    "Unlimited symbols".to_string(),
                    "Full historical archive".to_string(),
                    "Dedicated connection".to_string(),
                    "24/7 support".to_string(),
                ],
                api_calls: None,
                support_level: "Premium".to_string(),
            },
        ],
        features: vec![
            "Low Latency".to_string(),
            "Global Coverage".to_string(),
            "Historical Data".to_string(),
            "Real-time".to_string(),
            "Multi-Asset".to_string(),
        ],
        tags: vec!["market-data".to_string(), "real-time".to_string(), "equities".to_string(), "forex".to_string(), "crypto".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string(), "WebSocket".to_string(), "FIX".to_string()],
        avg_rating: 4.7,
        total_reviews: 203,
        total_installs: 8920,
        is_verified: true,
        is_featured: true,
        created_at: Utc::now() - chrono::Duration::days(400),
        updated_at: Utc::now() - chrono::Duration::days(1),
        status: "active".to_string(),
    };

    let service4 = AgentService {
        id: "svc-004".to_string(),
        name: "RiskMatrix".to_string(),
        slug: "riskmatrix".to_string(),
        short_description: "Comprehensive portfolio risk analytics and VaR calculations".to_string(),
        long_description: r#"RiskMatrix provides institutional-grade risk management tools for portfolio managers, risk officers, and compliance teams.

## Risk Metrics

- **Value at Risk (VaR)**: Historical, parametric, and Monte Carlo methods
- **Expected Shortfall**: Conditional VaR for tail risk assessment
- **Greeks**: Full options analytics including delta, gamma, vega, theta
- **Stress Testing**: Pre-built and custom scenario analysis
- **Factor Analysis**: Decompose risk by sector, style, geography

## Compliance

Built-in reports for Basel III/IV, FRTB, Solvency II, and other regulatory frameworks.

## Integration

Works seamlessly with major portfolio management systems and order management systems."#.to_string(),
        category_id: "risk-management".to_string(),
        agent_id: "agent-002".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=riskmatrix".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1551288049-bebda4e38f71?w=1200".to_string()),
        website_url: Some("https://securefinance.ai/riskmatrix".to_string()),
        documentation_url: Some("https://docs.securefinance.ai/riskmatrix".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Professional".to_string(),
                price: 399.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Basic VaR calculations".to_string(),
                    "Up to 100 positions".to_string(),
                    "Daily risk reports".to_string(),
                    "Email support".to_string(),
                ],
                api_calls: Some(50000),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "Enterprise".to_string(),
                price: 1499.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Full risk suite".to_string(),
                    "Unlimited positions".to_string(),
                    "Real-time monitoring".to_string(),
                    "Custom scenarios".to_string(),
                    "Regulatory reports".to_string(),
                    "Dedicated support".to_string(),
                ],
                api_calls: None,
                support_level: "Dedicated".to_string(),
            },
        ],
        features: vec![
            "VaR".to_string(),
            "Stress Testing".to_string(),
            "Regulatory".to_string(),
            "Real-time".to_string(),
            "Factor Analysis".to_string(),
        ],
        tags: vec!["risk".to_string(), "var".to_string(), "compliance".to_string(), "analytics".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string(), "GraphQL".to_string()],
        avg_rating: 4.6,
        total_reviews: 67,
        total_installs: 2100,
        is_verified: true,
        is_featured: false,
        created_at: Utc::now() - chrono::Duration::days(220),
        updated_at: Utc::now() - chrono::Duration::days(10),
        status: "active".to_string(),
    };

    let service5 = AgentService {
        id: "svc-005".to_string(),
        name: "KYCVerify".to_string(),
        slug: "kycverify".to_string(),
        short_description: "Automated identity verification and AML screening".to_string(),
        long_description: r#"KYCVerify streamlines customer onboarding with automated identity verification, document validation, and AML/sanctions screening.

## Verification Methods

- **Document Verification**: Passports, driver's licenses, national IDs from 200+ countries
- **Biometric Matching**: Facial recognition with liveness detection
- **Data Verification**: Cross-reference with credit bureaus, utility records
- **AML Screening**: PEP lists, sanctions, adverse media

## Compliance

Meet KYC requirements for banking, securities, insurance, and crypto regulations worldwide.

## Developer Experience

Simple API with webhooks, customizable workflows, and white-label options for seamless integration into your customer journey."#.to_string(),
        category_id: "kyc-aml".to_string(),
        agent_id: "agent-002".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=kycverify".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1450101499163-c8848c66ca85?w=1200".to_string()),
        website_url: Some("https://securefinance.ai/kycverify".to_string()),
        documentation_url: Some("https://docs.securefinance.ai/kycverify".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Startup".to_string(),
                price: 149.0,
                billing_period: "month".to_string(),
                features: vec![
                    "100 verifications/month".to_string(),
                    "Document verification".to_string(),
                    "Basic AML screening".to_string(),
                    "Dashboard access".to_string(),
                ],
                api_calls: Some(100),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "Business".to_string(),
                price: 499.0,
                billing_period: "month".to_string(),
                features: vec![
                    "1000 verifications/month".to_string(),
                    "Biometric matching".to_string(),
                    "Enhanced AML".to_string(),
                    "Custom workflows".to_string(),
                    "Priority support".to_string(),
                ],
                api_calls: Some(1000),
                support_level: "Priority".to_string(),
            },
            PricingTier {
                name: "Enterprise".to_string(),
                price: 1999.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Unlimited verifications".to_string(),
                    "White-label solution".to_string(),
                    "On-premise option".to_string(),
                    "Custom integrations".to_string(),
                    "Dedicated manager".to_string(),
                ],
                api_calls: None,
                support_level: "Dedicated".to_string(),
            },
        ],
        features: vec![
            "Identity Verification".to_string(),
            "AML Screening".to_string(),
            "Biometrics".to_string(),
            "Document OCR".to_string(),
            "Global Coverage".to_string(),
        ],
        tags: vec!["kyc".to_string(), "aml".to_string(), "identity".to_string(), "compliance".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string()],
        avg_rating: 4.7,
        total_reviews: 156,
        total_installs: 4500,
        is_verified: true,
        is_featured: false,
        created_at: Utc::now() - chrono::Duration::days(300),
        updated_at: Utc::now() - chrono::Duration::days(7),
        status: "active".to_string(),
    };

    let service6 = AgentService {
        id: "svc-006".to_string(),
        name: "SmartRebalancer".to_string(),
        slug: "smartrebalancer".to_string(),
        short_description: "Intelligent portfolio rebalancing and tax-loss harvesting".to_string(),
        long_description: r#"SmartRebalancer automates portfolio management with intelligent rebalancing algorithms and tax optimization strategies.

## Core Features

- **Threshold-based Rebalancing**: Automatic triggers when allocations drift
- **Tax-Loss Harvesting**: Identify and execute tax-saving opportunities
- **Cash Flow Optimization**: Smart handling of deposits and withdrawals
- **Model Portfolios**: Pre-built and custom allocation templates
- **Multi-Account**: Unified management across account types

## Tax Optimization

Our algorithms consider wash sale rules, short-term vs long-term gains, and lot selection to minimize tax impact.

## Integrations

Connect with major custodians including Schwab, Fidelity, Pershing, and Interactive Brokers."#.to_string(),
        category_id: "portfolio-management".to_string(),
        agent_id: "agent-001".to_string(),
        icon_url: "https://api.dicebear.com/7.x/shapes/svg?seed=rebalancer".to_string(),
        banner_url: Some("https://images.unsplash.com/photo-1579532537598-459ecdaf39cc?w=1200".to_string()),
        website_url: Some("https://quantalpha.io/rebalancer".to_string()),
        documentation_url: Some("https://docs.quantalpha.io/rebalancer".to_string()),
        pricing_tiers: vec![
            PricingTier {
                name: "Advisor".to_string(),
                price: 199.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Up to 50 accounts".to_string(),
                    "Basic rebalancing".to_string(),
                    "Model portfolios".to_string(),
                    "Email support".to_string(),
                ],
                api_calls: Some(10000),
                support_level: "Standard".to_string(),
            },
            PricingTier {
                name: "RIA".to_string(),
                price: 599.0,
                billing_period: "month".to_string(),
                features: vec![
                    "Up to 500 accounts".to_string(),
                    "Tax-loss harvesting".to_string(),
                    "Custom models".to_string(),
                    "Custodian integrations".to_string(),
                    "Priority support".to_string(),
                ],
                api_calls: Some(100000),
                support_level: "Priority".to_string(),
            },
        ],
        features: vec![
            "Rebalancing".to_string(),
            "Tax Optimization".to_string(),
            "Multi-Account".to_string(),
            "Model Portfolios".to_string(),
            "Automation".to_string(),
        ],
        tags: vec!["portfolio".to_string(), "rebalancing".to_string(), "tax".to_string(), "wealth".to_string()],
        integration_type: "API".to_string(),
        supported_protocols: vec!["REST".to_string()],
        avg_rating: 4.5,
        total_reviews: 45,
        total_installs: 1800,
        is_verified: true,
        is_featured: false,
        created_at: Utc::now() - chrono::Duration::days(120),
        updated_at: Utc::now() - chrono::Duration::days(15),
        status: "active".to_string(),
    };

    services.insert("svc-001".to_string(), service1);
    services.insert("svc-002".to_string(), service2);
    services.insert("svc-003".to_string(), service3);
    services.insert("svc-004".to_string(), service4);
    services.insert("svc-005".to_string(), service5);
    services.insert("svc-006".to_string(), service6);
    *state.services.write().await = services;

    // Initialize reviews
    let mut reviews = HashMap::new();

    reviews.insert("svc-001".to_string(), vec![
        Review {
            id: "rev-001".to_string(),
            service_id: "svc-001".to_string(),
            agent_id: "agent-001".to_string(),
            user_id: "user-001".to_string(),
            user_name: "Michael Chen".to_string(),
            user_avatar: Some("https://api.dicebear.com/7.x/avataaars/svg?seed=michael".to_string()),
            rating: 5,
            title: "Game changer for our trading desk".to_string(),
            content: "We've been using AlphaTrader Pro for 6 months and the results have been exceptional. The ML-based strategy adaptation has significantly improved our hit rate during volatile markets.".to_string(),
            helpful_count: 24,
            created_at: Utc::now() - chrono::Duration::days(30),
            verified_purchase: true,
        },
        Review {
            id: "rev-002".to_string(),
            service_id: "svc-001".to_string(),
            agent_id: "agent-001".to_string(),
            user_id: "user-002".to_string(),
            user_name: "Sarah Johnson".to_string(),
            user_avatar: Some("https://api.dicebear.com/7.x/avataaars/svg?seed=sarah".to_string()),
            rating: 4,
            title: "Great platform, steep learning curve".to_string(),
            content: "The capabilities are impressive but it took our team a few weeks to fully understand all the features. Documentation could be more comprehensive. That said, the results speak for themselves.".to_string(),
            helpful_count: 18,
            created_at: Utc::now() - chrono::Duration::days(45),
            verified_purchase: true,
        },
    ]);

    reviews.insert("svc-002".to_string(), vec![
        Review {
            id: "rev-003".to_string(),
            service_id: "svc-002".to_string(),
            agent_id: "agent-002".to_string(),
            user_id: "user-003".to_string(),
            user_name: "David Park".to_string(),
            user_avatar: Some("https://api.dicebear.com/7.x/avataaars/svg?seed=david".to_string()),
            rating: 5,
            title: "Blocked $50K in fraudulent transactions first week".to_string(),
            content: "Incredible accuracy. We integrated FraudShield in a single afternoon and it immediately started catching fraud our old system missed. The false positive rate is remarkably low.".to_string(),
            helpful_count: 42,
            created_at: Utc::now() - chrono::Duration::days(20),
            verified_purchase: true,
        },
    ]);

    *state.reviews.write().await = reviews;
}

// ============================================================================
// API Handlers
// ============================================================================

async fn root() -> Html<&'static str> {
    Html(include_str!("dashboard.html"))
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "openibank-marketplace",
        "version": "0.1.0"
    }))
}

async fn get_stats(State(state): State<Arc<AppState>>) -> Json<ApiResponse<MarketplaceStats>> {
    let services = state.services.read().await;
    let agents = state.agents.read().await;
    let categories = state.categories.read().await;

    let total_installs: u64 = services.values().map(|s| s.total_installs).sum();

    Json(ApiResponse {
        success: true,
        data: Some(MarketplaceStats {
            total_services: services.len() as u64,
            total_agents: agents.len() as u64,
            total_installs,
            categories: categories.clone(),
        }),
        error: None,
        meta: None,
    })
}

async fn list_services(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Json<ApiResponse<Vec<AgentService>>> {
    let services = state.services.read().await;
    let mut result: Vec<AgentService> = services.values().cloned().collect();

    // Apply filters
    if let Some(q) = &query.q {
        let q_lower = q.to_lowercase();
        result.retain(|s| {
            s.name.to_lowercase().contains(&q_lower)
                || s.short_description.to_lowercase().contains(&q_lower)
                || s.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
        });
    }

    if let Some(category) = &query.category {
        result.retain(|s| &s.category_id == category);
    }

    if let Some(min_rating) = query.min_rating {
        result.retain(|s| s.avg_rating >= min_rating);
    }

    if query.verified_only.unwrap_or(false) {
        result.retain(|s| s.is_verified);
    }

    // Sort
    match query.sort_by.as_deref() {
        Some("rating") => result.sort_by(|a, b| b.avg_rating.partial_cmp(&a.avg_rating).unwrap()),
        Some("installs") => result.sort_by(|a, b| b.total_installs.cmp(&a.total_installs)),
        Some("newest") => result.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
        _ => result.sort_by(|a, b| b.total_installs.cmp(&a.total_installs)),
    }

    let total = result.len() as u64;
    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);
    let offset = ((page - 1) * limit) as usize;
    let paginated: Vec<AgentService> = result.into_iter().skip(offset).take(limit as usize).collect();

    Json(ApiResponse {
        success: true,
        data: Some(paginated),
        error: None,
        meta: Some(ResponseMeta {
            page,
            limit,
            total,
            total_pages: ((total as f64) / (limit as f64)).ceil() as u32,
        }),
    })
}

async fn get_service(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<AgentService>>, StatusCode> {
    let services = state.services.read().await;

    match services.get(&id) {
        Some(service) => Ok(Json(ApiResponse {
            success: true,
            data: Some(service.clone()),
            error: None,
            meta: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_featured(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<AgentService>>> {
    let services = state.services.read().await;
    let featured: Vec<AgentService> = services
        .values()
        .filter(|s| s.is_featured)
        .cloned()
        .collect();

    Json(ApiResponse {
        success: true,
        data: Some(featured),
        error: None,
        meta: None,
    })
}

async fn list_categories(State(state): State<Arc<AppState>>) -> Json<ApiResponse<Vec<ServiceCategory>>> {
    let categories = state.categories.read().await;

    Json(ApiResponse {
        success: true,
        data: Some(categories.clone()),
        error: None,
        meta: None,
    })
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ApiResponse<Agent>>, StatusCode> {
    let agents = state.agents.read().await;

    match agents.get(&id) {
        Some(agent) => Ok(Json(ApiResponse {
            success: true,
            data: Some(agent.clone()),
            error: None,
            meta: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_agent_services(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<AgentService>>> {
    let services = state.services.read().await;
    let agent_services: Vec<AgentService> = services
        .values()
        .filter(|s| s.agent_id == id)
        .cloned()
        .collect();

    Json(ApiResponse {
        success: true,
        data: Some(agent_services),
        error: None,
        meta: None,
    })
}

async fn get_agent_reviews(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<Review>>> {
    let reviews = state.reviews.read().await;
    let services = state.services.read().await;

    // Get all services for this agent
    let agent_service_ids: Vec<String> = services
        .values()
        .filter(|s| s.agent_id == id)
        .map(|s| s.id.clone())
        .collect();

    // Get reviews for those services
    let mut agent_reviews = Vec::new();
    for service_id in agent_service_ids {
        if let Some(service_reviews) = reviews.get(&service_id) {
            agent_reviews.extend(service_reviews.clone());
        }
    }

    Json(ApiResponse {
        success: true,
        data: Some(agent_reviews),
        error: None,
        meta: None,
    })
}

async fn get_service_reviews(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<Vec<Review>>> {
    let reviews = state.reviews.read().await;

    let service_reviews = reviews.get(&id).cloned().unwrap_or_default();

    Json(ApiResponse {
        success: true,
        data: Some(service_reviews),
        error: None,
        meta: None,
    })
}

async fn create_review(
    State(state): State<Arc<AppState>>,
    Path(service_id): Path<String>,
    Json(req): Json<CreateReviewRequest>,
) -> Result<Json<ApiResponse<Review>>, StatusCode> {
    let services = state.services.read().await;

    let service = match services.get(&service_id) {
        Some(s) => s.clone(),
        None => return Err(StatusCode::NOT_FOUND),
    };
    drop(services);

    let review = Review {
        id: format!("rev-{}", Uuid::new_v4()),
        service_id: service_id.clone(),
        agent_id: service.agent_id,
        user_id: "demo-user".to_string(),
        user_name: "Demo User".to_string(),
        user_avatar: Some("https://api.dicebear.com/7.x/avataaars/svg?seed=demo".to_string()),
        rating: req.rating.min(5).max(1),
        title: req.title,
        content: req.content,
        helpful_count: 0,
        created_at: Utc::now(),
        verified_purchase: false,
    };

    let mut reviews = state.reviews.write().await;
    reviews.entry(service_id).or_default().push(review.clone());

    Ok(Json(ApiResponse {
        success: true,
        data: Some(review),
        error: None,
        meta: None,
    }))
}

async fn search_services(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Json<ApiResponse<Vec<AgentService>>> {
    list_services(State(state), Query(query)).await
}

async fn create_service(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateServiceRequest>,
) -> Result<Json<ApiResponse<AgentService>>, StatusCode> {
    let id = format!("svc-{}", Uuid::new_v4());
    let slug = req.name.to_lowercase().replace(' ', "-");

    let service = AgentService {
        id: id.clone(),
        name: req.name,
        slug,
        short_description: req.short_description,
        long_description: req.long_description,
        category_id: req.category_id,
        agent_id: "demo-agent".to_string(),
        icon_url: req.icon_url,
        banner_url: None,
        website_url: None,
        documentation_url: None,
        pricing_tiers: req.pricing_tiers,
        features: req.features,
        tags: req.tags,
        integration_type: req.integration_type,
        supported_protocols: req.supported_protocols,
        avg_rating: 0.0,
        total_reviews: 0,
        total_installs: 0,
        is_verified: false,
        is_featured: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        status: "pending".to_string(),
    };

    let mut services = state.services.write().await;
    services.insert(id, service.clone());

    Ok(Json(ApiResponse {
        success: true,
        data: Some(service),
        error: None,
        meta: None,
    }))
}

// ============================================================================
// Main Application
// ============================================================================

#[tokio::main]
async fn main() {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Initialize state
    let state = Arc::new(AppState::new());
    init_sample_data(&state).await;

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // Build router
    let app = Router::new()
        // Dashboard
        .route("/", get(root))
        .route("/health", get(health_check))
        // API routes
        .route("/api/stats", get(get_stats))
        .route("/api/services", get(list_services).post(create_service))
        .route("/api/services/{id}", get(get_service))
        .route("/api/services/{id}/reviews", get(get_service_reviews).post(create_review))
        .route("/api/featured", get(get_featured))
        .route("/api/categories", get(list_categories))
        .route("/api/agents/{id}", get(get_agent))
        .route("/api/agents/{id}/services", get(get_agent_services))
        .route("/api/agents/{id}/reviews", get(get_agent_reviews))
        .route("/api/search", get(search_services))
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3007));
    info!("OpenIBank Marketplace Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
