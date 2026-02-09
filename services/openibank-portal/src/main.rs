//! OpeniBank Portal Dashboard
//!
//! A world-class unified entry point to all OpeniBank services.
//! Features real-time health monitoring, SSE updates, and a professional fintech UI.

use axum::{
    extract::State,
    response::{Html, Sse},
    routing::get,
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceConfig {
    name: String,
    display_name: String,
    description: String,
    port: u16,
    health_endpoint: String,
    icon: String,
    color: String,
    docs_url: Option<String>,
}

/// Service health status
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServiceStatus {
    name: String,
    display_name: String,
    description: String,
    port: u16,
    status: String,
    latency_ms: Option<u64>,
    last_check: String,
    icon: String,
    color: String,
    docs_url: Option<String>,
    error: Option<String>,
}

/// System metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemMetrics {
    total_services: usize,
    online_services: usize,
    offline_services: usize,
    avg_latency_ms: f64,
    uptime_percentage: f64,
    last_updated: String,
}

/// Application state
#[derive(Debug)]
struct AppState {
    services: Vec<ServiceConfig>,
    status_cache: RwLock<Vec<ServiceStatus>>,
    metrics_cache: RwLock<SystemMetrics>,
}

impl AppState {
    fn new() -> Self {
        let services = vec![
            ServiceConfig {
                name: "playground".to_string(),
                display_name: "API Playground".to_string(),
                description: "Interactive API testing and exploration environment".to_string(),
                port: 8080,
                health_endpoint: "/health".to_string(),
                icon: "M14.447 3.027a.75.75 0 01.527.92l-4.5 16.5a.75.75 0 01-1.448-.394l4.5-16.5a.75.75 0 01.921-.526zM16.72 6.22a.75.75 0 011.06 0l5.25 5.25a.75.75 0 010 1.06l-5.25 5.25a.75.75 0 11-1.06-1.06L21.44 12l-4.72-4.72a.75.75 0 010-1.06zm-9.44 0a.75.75 0 010 1.06L2.56 12l4.72 4.72a.75.75 0 11-1.06 1.06L.97 12.53a.75.75 0 010-1.06l5.25-5.25a.75.75 0 011.06 0z".to_string(),
                color: "#6366f1".to_string(),
                docs_url: Some("/docs/playground".to_string()),
            },
            ServiceConfig {
                name: "exchange".to_string(),
                display_name: "Currency Exchange".to_string(),
                description: "Real-time foreign exchange rates and conversions".to_string(),
                port: 8888,
                health_endpoint: "/health".to_string(),
                icon: "M12 6v12m-3-2.818l.879.659c1.171.879 3.07.879 4.242 0 1.172-.879 1.172-2.303 0-3.182C13.536 12.219 12.768 12 12 12c-.725 0-1.45-.22-2.003-.659-1.106-.879-1.106-2.303 0-3.182s2.9-.879 4.006 0l.415.33M21 12a9 9 0 11-18 0 9 9 0 0118 0z".to_string(),
                color: "#10b981".to_string(),
                docs_url: Some("/docs/exchange".to_string()),
            },
            ServiceConfig {
                name: "api".to_string(),
                display_name: "Core API".to_string(),
                description: "Primary banking API with accounts, transfers, and more".to_string(),
                port: 3000,
                health_endpoint: "/health".to_string(),
                icon: "M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4".to_string(),
                color: "#f59e0b".to_string(),
                docs_url: Some("/docs/api".to_string()),
            },
            ServiceConfig {
                name: "issuer".to_string(),
                display_name: "Card Issuer".to_string(),
                description: "Card issuance, management, and transaction processing".to_string(),
                port: 8081,
                health_endpoint: "/health".to_string(),
                icon: "M2.25 8.25h19.5M2.25 9h19.5m-16.5 5.25h6m-6 2.25h3m-3.75 3h15a2.25 2.25 0 002.25-2.25V6.75A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25v10.5A2.25 2.25 0 004.5 19.5z".to_string(),
                color: "#ec4899".to_string(),
                docs_url: Some("/docs/issuer".to_string()),
            },
        ];

        let initial_statuses: Vec<ServiceStatus> = services
            .iter()
            .map(|s| ServiceStatus {
                name: s.name.clone(),
                display_name: s.display_name.clone(),
                description: s.description.clone(),
                port: s.port,
                status: "unknown".to_string(),
                latency_ms: None,
                last_check: chrono::Utc::now().to_rfc3339(),
                icon: s.icon.clone(),
                color: s.color.clone(),
                docs_url: s.docs_url.clone(),
                error: None,
            })
            .collect();

        Self {
            services,
            status_cache: RwLock::new(initial_statuses),
            metrics_cache: RwLock::new(SystemMetrics {
                total_services: 4,
                online_services: 0,
                offline_services: 4,
                avg_latency_ms: 0.0,
                uptime_percentage: 0.0,
                last_updated: chrono::Utc::now().to_rfc3339(),
            }),
        }
    }
}

/// Check health of a single service
async fn check_service_health(config: &ServiceConfig) -> ServiceStatus {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    let url = format!("http://localhost:{}{}", config.port, config.health_endpoint);
    let start = std::time::Instant::now();

    let (status, latency_ms, error) = match client.get(&url).send().await {
        Ok(resp) => {
            let latency = start.elapsed().as_millis() as u64;
            if resp.status().is_success() {
                ("online".to_string(), Some(latency), None)
            } else {
                (
                    "degraded".to_string(),
                    Some(latency),
                    Some(format!("HTTP {}", resp.status())),
                )
            }
        }
        Err(e) => {
            let error_msg = if e.is_connect() {
                "Connection refused".to_string()
            } else if e.is_timeout() {
                "Request timeout".to_string()
            } else {
                e.to_string()
            };
            ("offline".to_string(), None, Some(error_msg))
        }
    };

    ServiceStatus {
        name: config.name.clone(),
        display_name: config.display_name.clone(),
        description: config.description.clone(),
        port: config.port,
        status,
        latency_ms,
        last_check: chrono::Utc::now().to_rfc3339(),
        icon: config.icon.clone(),
        color: config.color.clone(),
        docs_url: config.docs_url.clone(),
        error,
    }
}

/// Update all service statuses
async fn update_all_statuses(state: &AppState) {
    let mut statuses = Vec::new();

    for config in &state.services {
        let status = check_service_health(config).await;
        statuses.push(status);
    }

    let online_count = statuses.iter().filter(|s| s.status == "online").count();
    let total_latency: u64 = statuses.iter().filter_map(|s| s.latency_ms).sum();
    let latency_count = statuses.iter().filter(|s| s.latency_ms.is_some()).count();
    let avg_latency = if latency_count > 0 {
        total_latency as f64 / latency_count as f64
    } else {
        0.0
    };

    let metrics = SystemMetrics {
        total_services: statuses.len(),
        online_services: online_count,
        offline_services: statuses.len() - online_count,
        avg_latency_ms: avg_latency,
        uptime_percentage: (online_count as f64 / statuses.len() as f64) * 100.0,
        last_updated: chrono::Utc::now().to_rfc3339(),
    };

    *state.status_cache.write().await = statuses;
    *state.metrics_cache.write().await = metrics;
}

/// Background task to periodically update service statuses
async fn health_check_task(state: Arc<AppState>) {
    loop {
        update_all_statuses(&state).await;
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

/// SSE event for real-time updates
#[derive(Debug, Clone, Serialize)]
struct SseUpdate {
    event_type: String,
    services: Vec<ServiceStatus>,
    metrics: SystemMetrics,
    timestamp: String,
}

/// Handler for SSE stream
async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let stream = stream::unfold(state, |state| async move {
        tokio::time::sleep(Duration::from_secs(5)).await;

        let services = state.status_cache.read().await.clone();
        let metrics = state.metrics_cache.read().await.clone();

        let update = SseUpdate {
            event_type: "status_update".to_string(),
            services,
            metrics,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        let event = axum::response::sse::Event::default()
            .event("status")
            .data(serde_json::to_string(&update).unwrap_or_default());

        Some((Ok(event), state))
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

/// API handler for service statuses
async fn api_status_handler(State(state): State<Arc<AppState>>) -> Json<Vec<ServiceStatus>> {
    let statuses = state.status_cache.read().await.clone();
    Json(statuses)
}

/// API handler for system metrics
async fn api_metrics_handler(State(state): State<Arc<AppState>>) -> Json<SystemMetrics> {
    let metrics = state.metrics_cache.read().await.clone();
    Json(metrics)
}

/// Health check endpoint for this service
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "openibank-portal",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Main dashboard HTML
async fn dashboard_handler(State(state): State<Arc<AppState>>) -> Html<String> {
    let services = state.status_cache.read().await.clone();
    let metrics = state.metrics_cache.read().await.clone();

    Html(generate_dashboard_html(&services, &metrics))
}

/// Generate the complete dashboard HTML
fn generate_dashboard_html(services: &[ServiceStatus], metrics: &SystemMetrics) -> String {
    let service_cards: String = services
        .iter()
        .map(|s| generate_service_card(s))
        .collect::<Vec<_>>()
        .join("\n");

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>OpeniBank Portal | Developer Dashboard</title>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700;800&display=swap" rel="stylesheet">
    <style>
        :root {{
            --bg-primary: #0a0a0f;
            --bg-secondary: #12121a;
            --bg-tertiary: #1a1a24;
            --bg-card: rgba(26, 26, 36, 0.6);
            --bg-glass: rgba(255, 255, 255, 0.03);
            --border-primary: rgba(255, 255, 255, 0.06);
            --border-secondary: rgba(255, 255, 255, 0.1);
            --text-primary: #ffffff;
            --text-secondary: rgba(255, 255, 255, 0.7);
            --text-tertiary: rgba(255, 255, 255, 0.5);
            --accent-primary: #6366f1;
            --accent-secondary: #8b5cf6;
            --success: #10b981;
            --warning: #f59e0b;
            --error: #ef4444;
            --gradient-1: linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%);
            --gradient-2: linear-gradient(135deg, #10b981 0%, #059669 100%);
            --gradient-3: linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%);
            --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
            --shadow-md: 0 4px 6px -1px rgba(0, 0, 0, 0.3), 0 2px 4px -2px rgba(0, 0, 0, 0.2);
            --shadow-lg: 0 10px 15px -3px rgba(0, 0, 0, 0.4), 0 4px 6px -4px rgba(0, 0, 0, 0.3);
            --shadow-xl: 0 20px 25px -5px rgba(0, 0, 0, 0.5), 0 8px 10px -6px rgba(0, 0, 0, 0.4);
            --shadow-glow: 0 0 40px rgba(99, 102, 241, 0.15);
        }}

        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}

        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: var(--bg-primary);
            color: var(--text-primary);
            min-height: 100vh;
            line-height: 1.6;
            overflow-x: hidden;
        }}

        /* Animated background */
        .bg-animation {{
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            z-index: -1;
            overflow: hidden;
        }}

        .bg-animation::before {{
            content: '';
            position: absolute;
            top: -50%;
            left: -50%;
            width: 200%;
            height: 200%;
            background: radial-gradient(circle at 20% 80%, rgba(99, 102, 241, 0.08) 0%, transparent 50%),
                        radial-gradient(circle at 80% 20%, rgba(139, 92, 246, 0.08) 0%, transparent 50%),
                        radial-gradient(circle at 40% 40%, rgba(16, 185, 129, 0.05) 0%, transparent 40%);
            animation: bgMove 30s ease-in-out infinite;
        }}

        @keyframes bgMove {{
            0%, 100% {{ transform: translate(0, 0) rotate(0deg); }}
            33% {{ transform: translate(2%, 2%) rotate(1deg); }}
            66% {{ transform: translate(-1%, 1%) rotate(-1deg); }}
        }}

        /* Grid pattern overlay */
        .grid-pattern {{
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            z-index: -1;
            background-image:
                linear-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px),
                linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
            background-size: 60px 60px;
            mask-image: radial-gradient(ellipse at center, black 0%, transparent 70%);
        }}

        /* Header */
        .header {{
            position: sticky;
            top: 0;
            z-index: 100;
            background: rgba(10, 10, 15, 0.8);
            backdrop-filter: blur(20px);
            border-bottom: 1px solid var(--border-primary);
            padding: 1rem 2rem;
        }}

        .header-content {{
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }}

        .logo {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .logo-icon {{
            width: 40px;
            height: 40px;
            background: var(--gradient-1);
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            box-shadow: var(--shadow-glow);
        }}

        .logo-icon svg {{
            width: 24px;
            height: 24px;
            color: white;
        }}

        .logo-text {{
            font-size: 1.5rem;
            font-weight: 700;
            background: linear-gradient(135deg, #fff 0%, rgba(255,255,255,0.7) 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        .logo-badge {{
            font-size: 0.65rem;
            font-weight: 600;
            padding: 0.2rem 0.5rem;
            background: var(--gradient-1);
            border-radius: 4px;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        .header-nav {{
            display: flex;
            align-items: center;
            gap: 1rem;
        }}

        .nav-link {{
            color: var(--text-secondary);
            text-decoration: none;
            font-size: 0.875rem;
            font-weight: 500;
            padding: 0.5rem 1rem;
            border-radius: 8px;
            transition: all 0.2s ease;
        }}

        .nav-link:hover {{
            color: var(--text-primary);
            background: var(--bg-glass);
        }}

        .nav-link.active {{
            color: var(--text-primary);
            background: var(--bg-tertiary);
        }}

        .header-actions {{
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .btn {{
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.625rem 1.25rem;
            font-size: 0.875rem;
            font-weight: 500;
            border-radius: 8px;
            border: none;
            cursor: pointer;
            transition: all 0.2s ease;
            text-decoration: none;
        }}

        .btn-primary {{
            background: var(--gradient-1);
            color: white;
            box-shadow: var(--shadow-md), 0 0 20px rgba(99, 102, 241, 0.3);
        }}

        .btn-primary:hover {{
            transform: translateY(-1px);
            box-shadow: var(--shadow-lg), 0 0 30px rgba(99, 102, 241, 0.4);
        }}

        .btn-secondary {{
            background: var(--bg-tertiary);
            color: var(--text-primary);
            border: 1px solid var(--border-secondary);
        }}

        .btn-secondary:hover {{
            background: rgba(255, 255, 255, 0.1);
            border-color: var(--border-secondary);
        }}

        .btn-icon {{
            width: 36px;
            height: 36px;
            padding: 0;
            display: flex;
            align-items: center;
            justify-content: center;
            background: var(--bg-tertiary);
            border: 1px solid var(--border-primary);
            border-radius: 8px;
        }}

        .btn-icon svg {{
            width: 18px;
            height: 18px;
            color: var(--text-secondary);
        }}

        /* Main container */
        .container {{
            max-width: 1600px;
            margin: 0 auto;
            padding: 2rem;
        }}

        /* Hero section */
        .hero {{
            text-align: center;
            padding: 3rem 0 4rem;
            position: relative;
        }}

        .hero-badge {{
            display: inline-flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.5rem 1rem;
            background: var(--bg-glass);
            border: 1px solid var(--border-primary);
            border-radius: 100px;
            font-size: 0.8rem;
            color: var(--text-secondary);
            margin-bottom: 1.5rem;
        }}

        .hero-badge-dot {{
            width: 6px;
            height: 6px;
            background: var(--success);
            border-radius: 50%;
            animation: pulse 2s ease-in-out infinite;
        }}

        @keyframes pulse {{
            0%, 100% {{ opacity: 1; transform: scale(1); }}
            50% {{ opacity: 0.5; transform: scale(1.2); }}
        }}

        .hero-title {{
            font-size: 3.5rem;
            font-weight: 800;
            line-height: 1.1;
            margin-bottom: 1rem;
            background: linear-gradient(135deg, #fff 0%, rgba(255,255,255,0.8) 50%, rgba(255,255,255,0.6) 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}

        .hero-subtitle {{
            font-size: 1.25rem;
            color: var(--text-secondary);
            max-width: 600px;
            margin: 0 auto 2rem;
            font-weight: 400;
        }}

        .hero-actions {{
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 1rem;
            flex-wrap: wrap;
        }}

        /* Metrics grid */
        .metrics-grid {{
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 1.5rem;
            margin-bottom: 3rem;
        }}

        @media (max-width: 1200px) {{
            .metrics-grid {{ grid-template-columns: repeat(2, 1fr); }}
        }}

        @media (max-width: 640px) {{
            .metrics-grid {{ grid-template-columns: 1fr; }}
        }}

        .metric-card {{
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 16px;
            padding: 1.5rem;
            position: relative;
            overflow: hidden;
            transition: all 0.3s ease;
        }}

        .metric-card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 1px;
            background: linear-gradient(90deg, transparent, rgba(255,255,255,0.1), transparent);
        }}

        .metric-card:hover {{
            transform: translateY(-2px);
            border-color: var(--border-secondary);
            box-shadow: var(--shadow-lg);
        }}

        .metric-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1rem;
        }}

        .metric-label {{
            font-size: 0.875rem;
            color: var(--text-tertiary);
            font-weight: 500;
        }}

        .metric-icon {{
            width: 40px;
            height: 40px;
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
        }}

        .metric-icon svg {{
            width: 20px;
            height: 20px;
        }}

        .metric-icon.primary {{
            background: rgba(99, 102, 241, 0.15);
            color: #818cf8;
        }}

        .metric-icon.success {{
            background: rgba(16, 185, 129, 0.15);
            color: #34d399;
        }}

        .metric-icon.warning {{
            background: rgba(245, 158, 11, 0.15);
            color: #fbbf24;
        }}

        .metric-icon.error {{
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
        }}

        .metric-value {{
            font-size: 2.5rem;
            font-weight: 700;
            line-height: 1;
            margin-bottom: 0.5rem;
        }}

        .metric-change {{
            display: inline-flex;
            align-items: center;
            gap: 0.25rem;
            font-size: 0.75rem;
            font-weight: 500;
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
        }}

        .metric-change.positive {{
            background: rgba(16, 185, 129, 0.15);
            color: #34d399;
        }}

        .metric-change.negative {{
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
        }}

        /* Section headers */
        .section-header {{
            display: flex;
            align-items: center;
            justify-content: space-between;
            margin-bottom: 1.5rem;
        }}

        .section-title {{
            font-size: 1.25rem;
            font-weight: 600;
            display: flex;
            align-items: center;
            gap: 0.75rem;
        }}

        .section-title-icon {{
            width: 32px;
            height: 32px;
            background: var(--gradient-1);
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
        }}

        .section-title-icon svg {{
            width: 18px;
            height: 18px;
            color: white;
        }}

        /* Services grid */
        .services-grid {{
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 1.5rem;
            margin-bottom: 3rem;
        }}

        @media (max-width: 900px) {{
            .services-grid {{ grid-template-columns: 1fr; }}
        }}

        .service-card {{
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 20px;
            padding: 1.75rem;
            position: relative;
            overflow: hidden;
            transition: all 0.3s ease;
        }}

        .service-card::before {{
            content: '';
            position: absolute;
            top: 0;
            left: 0;
            right: 0;
            height: 2px;
            background: linear-gradient(90deg, transparent, var(--card-color, var(--accent-primary)), transparent);
            opacity: 0;
            transition: opacity 0.3s ease;
        }}

        .service-card:hover {{
            transform: translateY(-4px);
            border-color: rgba(255, 255, 255, 0.1);
            box-shadow: var(--shadow-xl), 0 0 40px rgba(99, 102, 241, 0.1);
        }}

        .service-card:hover::before {{
            opacity: 1;
        }}

        .service-header {{
            display: flex;
            align-items: flex-start;
            gap: 1rem;
            margin-bottom: 1rem;
        }}

        .service-icon {{
            width: 48px;
            height: 48px;
            border-radius: 12px;
            display: flex;
            align-items: center;
            justify-content: center;
            flex-shrink: 0;
        }}

        .service-icon svg {{
            width: 24px;
            height: 24px;
            color: white;
        }}

        .service-info {{
            flex: 1;
            min-width: 0;
        }}

        .service-name {{
            font-size: 1.125rem;
            font-weight: 600;
            margin-bottom: 0.25rem;
        }}

        .service-description {{
            font-size: 0.875rem;
            color: var(--text-tertiary);
            line-height: 1.5;
        }}

        .service-status {{
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.375rem 0.75rem;
            border-radius: 100px;
            font-size: 0.75rem;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 0.05em;
        }}

        .service-status.online {{
            background: rgba(16, 185, 129, 0.15);
            color: #34d399;
        }}

        .service-status.offline {{
            background: rgba(239, 68, 68, 0.15);
            color: #f87171;
        }}

        .service-status.degraded {{
            background: rgba(245, 158, 11, 0.15);
            color: #fbbf24;
        }}

        .service-status.unknown {{
            background: rgba(107, 114, 128, 0.15);
            color: #9ca3af;
        }}

        .status-dot {{
            width: 6px;
            height: 6px;
            border-radius: 50%;
            background: currentColor;
        }}

        .service-status.online .status-dot {{
            animation: pulse 2s ease-in-out infinite;
        }}

        .service-meta {{
            display: flex;
            align-items: center;
            gap: 1.5rem;
            padding: 1rem 0;
            border-top: 1px solid var(--border-primary);
            border-bottom: 1px solid var(--border-primary);
            margin: 1rem 0;
        }}

        .service-meta-item {{
            display: flex;
            flex-direction: column;
            gap: 0.25rem;
        }}

        .service-meta-label {{
            font-size: 0.7rem;
            color: var(--text-tertiary);
            text-transform: uppercase;
            letter-spacing: 0.1em;
            font-weight: 500;
        }}

        .service-meta-value {{
            font-size: 0.9rem;
            font-weight: 600;
            color: var(--text-primary);
        }}

        .service-actions {{
            display: flex;
            gap: 0.75rem;
        }}

        .service-btn {{
            flex: 1;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            gap: 0.5rem;
            padding: 0.75rem 1rem;
            font-size: 0.8rem;
            font-weight: 500;
            border-radius: 10px;
            border: none;
            cursor: pointer;
            transition: all 0.2s ease;
            text-decoration: none;
        }}

        .service-btn-primary {{
            background: var(--card-color, var(--accent-primary));
            color: white;
        }}

        .service-btn-primary:hover {{
            filter: brightness(1.1);
            transform: translateY(-1px);
        }}

        .service-btn-secondary {{
            background: var(--bg-tertiary);
            color: var(--text-primary);
            border: 1px solid var(--border-secondary);
        }}

        .service-btn-secondary:hover {{
            background: rgba(255, 255, 255, 0.1);
        }}

        .service-btn svg {{
            width: 16px;
            height: 16px;
        }}

        /* Quick start section */
        .quickstart-section {{
            margin-bottom: 3rem;
        }}

        .quickstart-grid {{
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 1.5rem;
        }}

        @media (max-width: 900px) {{
            .quickstart-grid {{ grid-template-columns: 1fr; }}
        }}

        .quickstart-card {{
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 16px;
            padding: 1.5rem;
            transition: all 0.3s ease;
        }}

        .quickstart-card:hover {{
            border-color: var(--border-secondary);
            transform: translateY(-2px);
        }}

        .quickstart-step {{
            width: 28px;
            height: 28px;
            background: var(--gradient-1);
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 0.8rem;
            font-weight: 700;
            margin-bottom: 1rem;
        }}

        .quickstart-title {{
            font-size: 1rem;
            font-weight: 600;
            margin-bottom: 0.5rem;
        }}

        .quickstart-text {{
            font-size: 0.875rem;
            color: var(--text-tertiary);
            margin-bottom: 1rem;
            line-height: 1.6;
        }}

        .code-block {{
            background: var(--bg-primary);
            border: 1px solid var(--border-primary);
            border-radius: 8px;
            padding: 0.75rem 1rem;
            font-family: 'SF Mono', 'Fira Code', monospace;
            font-size: 0.8rem;
            color: #a5b4fc;
            overflow-x: auto;
            position: relative;
        }}

        .code-block::before {{
            content: '$';
            color: var(--text-tertiary);
            margin-right: 0.5rem;
        }}

        /* Docs section */
        .docs-grid {{
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 1rem;
        }}

        @media (max-width: 1200px) {{
            .docs-grid {{ grid-template-columns: repeat(2, 1fr); }}
        }}

        @media (max-width: 640px) {{
            .docs-grid {{ grid-template-columns: 1fr; }}
        }}

        .docs-card {{
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 12px;
            padding: 1.25rem;
            text-decoration: none;
            color: var(--text-primary);
            transition: all 0.2s ease;
            display: flex;
            align-items: center;
            gap: 1rem;
        }}

        .docs-card:hover {{
            border-color: var(--border-secondary);
            transform: translateY(-2px);
            background: rgba(255, 255, 255, 0.05);
        }}

        .docs-card-icon {{
            width: 40px;
            height: 40px;
            background: var(--bg-tertiary);
            border-radius: 10px;
            display: flex;
            align-items: center;
            justify-content: center;
            flex-shrink: 0;
        }}

        .docs-card-icon svg {{
            width: 20px;
            height: 20px;
            color: var(--text-secondary);
        }}

        .docs-card-content {{
            flex: 1;
            min-width: 0;
        }}

        .docs-card-title {{
            font-size: 0.9rem;
            font-weight: 600;
            margin-bottom: 0.125rem;
        }}

        .docs-card-description {{
            font-size: 0.75rem;
            color: var(--text-tertiary);
        }}

        /* Footer */
        .footer {{
            border-top: 1px solid var(--border-primary);
            padding: 2rem;
            margin-top: 4rem;
        }}

        .footer-content {{
            max-width: 1600px;
            margin: 0 auto;
            display: flex;
            align-items: center;
            justify-content: space-between;
        }}

        .footer-text {{
            font-size: 0.875rem;
            color: var(--text-tertiary);
        }}

        .footer-links {{
            display: flex;
            gap: 1.5rem;
        }}

        .footer-link {{
            font-size: 0.875rem;
            color: var(--text-tertiary);
            text-decoration: none;
            transition: color 0.2s ease;
        }}

        .footer-link:hover {{
            color: var(--text-primary);
        }}

        /* Connection status indicator */
        .connection-status {{
            position: fixed;
            bottom: 1.5rem;
            right: 1.5rem;
            display: flex;
            align-items: center;
            gap: 0.5rem;
            padding: 0.75rem 1rem;
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 100px;
            font-size: 0.75rem;
            color: var(--text-secondary);
            z-index: 1000;
            transition: all 0.3s ease;
        }}

        .connection-status.connected {{
            border-color: rgba(16, 185, 129, 0.3);
        }}

        .connection-status.disconnected {{
            border-color: rgba(239, 68, 68, 0.3);
        }}

        .connection-dot {{
            width: 8px;
            height: 8px;
            border-radius: 50%;
            animation: pulse 2s ease-in-out infinite;
        }}

        .connection-status.connected .connection-dot {{
            background: var(--success);
        }}

        .connection-status.disconnected .connection-dot {{
            background: var(--error);
        }}

        /* Toast notifications */
        .toast-container {{
            position: fixed;
            top: 1.5rem;
            right: 1.5rem;
            z-index: 1001;
            display: flex;
            flex-direction: column;
            gap: 0.75rem;
        }}

        .toast {{
            padding: 1rem 1.25rem;
            background: var(--bg-card);
            backdrop-filter: blur(20px);
            border: 1px solid var(--border-primary);
            border-radius: 12px;
            display: flex;
            align-items: center;
            gap: 0.75rem;
            font-size: 0.875rem;
            animation: slideIn 0.3s ease;
            box-shadow: var(--shadow-lg);
        }}

        @keyframes slideIn {{
            from {{
                transform: translateX(100%);
                opacity: 0;
            }}
            to {{
                transform: translateX(0);
                opacity: 1;
            }}
        }}

        .toast.success {{
            border-color: rgba(16, 185, 129, 0.3);
        }}

        .toast.error {{
            border-color: rgba(239, 68, 68, 0.3);
        }}

        .toast-icon {{
            width: 20px;
            height: 20px;
            flex-shrink: 0;
        }}

        .toast.success .toast-icon {{
            color: var(--success);
        }}

        .toast.error .toast-icon {{
            color: var(--error);
        }}

        /* Loading spinner */
        .loading-spinner {{
            width: 16px;
            height: 16px;
            border: 2px solid var(--border-primary);
            border-top-color: var(--accent-primary);
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }}

        @keyframes spin {{
            to {{ transform: rotate(360deg); }}
        }}

        /* Responsive adjustments */
        @media (max-width: 768px) {{
            .header {{
                padding: 1rem;
            }}

            .header-nav {{
                display: none;
            }}

            .container {{
                padding: 1rem;
            }}

            .hero-title {{
                font-size: 2rem;
            }}

            .hero-subtitle {{
                font-size: 1rem;
            }}

            .service-card {{
                padding: 1.25rem;
            }}

            .footer-content {{
                flex-direction: column;
                gap: 1rem;
                text-align: center;
            }}
        }}
    </style>
</head>
<body>
    <div class="bg-animation"></div>
    <div class="grid-pattern"></div>

    <!-- Header -->
    <header class="header">
        <div class="header-content">
            <div class="logo">
                <div class="logo-icon">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M12 2L2 7l10 5 10-5-10-5z"/>
                        <path d="M2 17l10 5 10-5"/>
                        <path d="M2 12l10 5 10-5"/>
                    </svg>
                </div>
                <span class="logo-text">OpeniBank</span>
                <span class="logo-badge">Portal</span>
            </div>
            <nav class="header-nav">
                <a href="#" class="nav-link active">Dashboard</a>
                <a href="#services" class="nav-link">Services</a>
                <a href="#docs" class="nav-link">Documentation</a>
                <a href="#quickstart" class="nav-link">Quick Start</a>
            </nav>
            <div class="header-actions">
                <button class="btn btn-icon" title="Refresh Status" onclick="refreshStatus()">
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                </button>
                <a href="https://github.com/openibank" class="btn btn-secondary" target="_blank">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/>
                    </svg>
                    GitHub
                </a>
                <a href="/docs" class="btn btn-primary">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                    View Docs
                </a>
            </div>
        </div>
    </header>

    <main class="container">
        <!-- Hero Section -->
        <section class="hero">
            <div class="hero-badge">
                <span class="hero-badge-dot"></span>
                <span>All systems operational</span>
            </div>
            <h1 class="hero-title">Developer Portal</h1>
            <p class="hero-subtitle">
                Your unified gateway to OpeniBank's open banking infrastructure.
                Build, test, and deploy financial applications with confidence.
            </p>
            <div class="hero-actions">
                <a href="#quickstart" class="btn btn-primary">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    Get Started
                </a>
                <a href="#services" class="btn btn-secondary">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M4 6h16M4 12h16M4 18h16"/>
                    </svg>
                    View Services
                </a>
            </div>
        </section>

        <!-- Metrics Grid -->
        <section class="metrics-grid">
            <div class="metric-card">
                <div class="metric-header">
                    <span class="metric-label">Total Services</span>
                    <div class="metric-icon primary">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M5 12h14M12 5l7 7-7 7"/>
                        </svg>
                    </div>
                </div>
                <div class="metric-value" id="metric-total">{total}</div>
                <span class="metric-change positive">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M7 14l5-5 5 5H7z"/>
                    </svg>
                    Active
                </span>
            </div>
            <div class="metric-card">
                <div class="metric-header">
                    <span class="metric-label">Online Services</span>
                    <div class="metric-icon success">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M5 13l4 4L19 7"/>
                        </svg>
                    </div>
                </div>
                <div class="metric-value" id="metric-online">{online}</div>
                <span class="metric-change positive">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M7 14l5-5 5 5H7z"/>
                    </svg>
                    {uptime:.1}% uptime
                </span>
            </div>
            <div class="metric-card">
                <div class="metric-header">
                    <span class="metric-label">Avg Latency</span>
                    <div class="metric-icon warning">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                    </div>
                </div>
                <div class="metric-value" id="metric-latency">{latency:.0}<span style="font-size: 1rem; font-weight: 400; color: var(--text-tertiary)">ms</span></div>
                <span class="metric-change positive">
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                        <path d="M7 14l5-5 5 5H7z"/>
                    </svg>
                    Optimal
                </span>
            </div>
            <div class="metric-card">
                <div class="metric-header">
                    <span class="metric-label">Offline Services</span>
                    <div class="metric-icon error">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </div>
                </div>
                <div class="metric-value" id="metric-offline">{offline}</div>
                <span class="metric-change {offline_class}">
                    {offline_status}
                </span>
            </div>
        </section>

        <!-- Services Section -->
        <section id="services">
            <div class="section-header">
                <h2 class="section-title">
                    <div class="section-title-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
                        </svg>
                    </div>
                    Platform Services
                </h2>
                <button class="btn btn-secondary" onclick="refreshStatus()">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    Refresh
                </button>
            </div>
            <div class="services-grid" id="services-grid">
                {service_cards}
            </div>
        </section>

        <!-- Quick Start Section -->
        <section id="quickstart" class="quickstart-section">
            <div class="section-header">
                <h2 class="section-title">
                    <div class="section-title-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
                        </svg>
                    </div>
                    Quick Start Guide
                </h2>
            </div>
            <div class="quickstart-grid">
                <div class="quickstart-card">
                    <div class="quickstart-step">1</div>
                    <h3 class="quickstart-title">Clone the Repository</h3>
                    <p class="quickstart-text">Get started by cloning the OpeniBank monorepo which contains all services and documentation.</p>
                    <div class="code-block">git clone https://github.com/openibank/openibank.git</div>
                </div>
                <div class="quickstart-card">
                    <div class="quickstart-step">2</div>
                    <h3 class="quickstart-title">Start Services</h3>
                    <p class="quickstart-text">Use Docker Compose or Cargo to spin up all services locally for development.</p>
                    <div class="code-block">docker-compose up -d</div>
                </div>
                <div class="quickstart-card">
                    <div class="quickstart-step">3</div>
                    <h3 class="quickstart-title">Explore the Playground</h3>
                    <p class="quickstart-text">Visit the API Playground to interactively test endpoints and explore the API.</p>
                    <div class="code-block">open http://localhost:8080</div>
                </div>
            </div>
        </section>

        <!-- Documentation Section -->
        <section id="docs">
            <div class="section-header">
                <h2 class="section-title">
                    <div class="section-title-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253"/>
                        </svg>
                    </div>
                    Documentation
                </h2>
            </div>
            <div class="docs-grid">
                <a href="/docs/getting-started" class="docs-card">
                    <div class="docs-card-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M13 10V3L4 14h7v7l9-11h-7z"/>
                        </svg>
                    </div>
                    <div class="docs-card-content">
                        <div class="docs-card-title">Getting Started</div>
                        <div class="docs-card-description">Quick setup guide</div>
                    </div>
                </a>
                <a href="/docs/api-reference" class="docs-card">
                    <div class="docs-card-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4"/>
                        </svg>
                    </div>
                    <div class="docs-card-content">
                        <div class="docs-card-title">API Reference</div>
                        <div class="docs-card-description">Complete API docs</div>
                    </div>
                </a>
                <a href="/docs/architecture" class="docs-card">
                    <div class="docs-card-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>
                        </svg>
                    </div>
                    <div class="docs-card-content">
                        <div class="docs-card-title">Architecture</div>
                        <div class="docs-card-description">System design</div>
                    </div>
                </a>
                <a href="/docs/changelog" class="docs-card">
                    <div class="docs-card-icon">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <path d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                        </svg>
                    </div>
                    <div class="docs-card-content">
                        <div class="docs-card-title">Changelog</div>
                        <div class="docs-card-description">Latest updates</div>
                    </div>
                </a>
            </div>
        </section>
    </main>

    <!-- Footer -->
    <footer class="footer">
        <div class="footer-content">
            <div class="footer-text">
                2024 OpeniBank. Open-source banking infrastructure.
            </div>
            <div class="footer-links">
                <a href="https://github.com/openibank" class="footer-link">GitHub</a>
                <a href="/docs" class="footer-link">Documentation</a>
                <a href="/status" class="footer-link">Status</a>
                <a href="/support" class="footer-link">Support</a>
            </div>
        </div>
    </footer>

    <!-- Connection Status Indicator -->
    <div class="connection-status connected" id="connection-status">
        <span class="connection-dot"></span>
        <span id="connection-text">Live updates active</span>
    </div>

    <!-- Toast Container -->
    <div class="toast-container" id="toast-container"></div>

    <script>
        // Real-time updates via SSE
        let eventSource = null;
        let reconnectAttempts = 0;
        const maxReconnectAttempts = 5;

        function connectSSE() {{
            if (eventSource) {{
                eventSource.close();
            }}

            eventSource = new EventSource('/api/sse');

            eventSource.onopen = () => {{
                reconnectAttempts = 0;
                updateConnectionStatus(true);
                console.log('SSE connection established');
            }};

            eventSource.addEventListener('status', (event) => {{
                try {{
                    const data = JSON.parse(event.data);
                    updateDashboard(data);
                }} catch (e) {{
                    console.error('Failed to parse SSE data:', e);
                }}
            }});

            eventSource.onerror = (error) => {{
                console.error('SSE error:', error);
                updateConnectionStatus(false);
                eventSource.close();

                if (reconnectAttempts < maxReconnectAttempts) {{
                    reconnectAttempts++;
                    const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), 30000);
                    console.log(`Reconnecting in ${{delay}}ms (attempt ${{reconnectAttempts}})`);
                    setTimeout(connectSSE, delay);
                }} else {{
                    showToast('Connection lost. Please refresh the page.', 'error');
                }}
            }};
        }}

        function updateConnectionStatus(connected) {{
            const statusEl = document.getElementById('connection-status');
            const textEl = document.getElementById('connection-text');

            if (connected) {{
                statusEl.classList.remove('disconnected');
                statusEl.classList.add('connected');
                textEl.textContent = 'Live updates active';
            }} else {{
                statusEl.classList.remove('connected');
                statusEl.classList.add('disconnected');
                textEl.textContent = 'Reconnecting...';
            }}
        }}

        function updateDashboard(data) {{
            // Update metrics
            if (data.metrics) {{
                document.getElementById('metric-total').textContent = data.metrics.total_services;
                document.getElementById('metric-online').textContent = data.metrics.online_services;
                document.getElementById('metric-offline').textContent = data.metrics.offline_services;
                document.getElementById('metric-latency').innerHTML =
                    `${{Math.round(data.metrics.avg_latency_ms)}}<span style="font-size: 1rem; font-weight: 400; color: var(--text-tertiary)">ms</span>`;
            }}

            // Update service cards
            if (data.services) {{
                data.services.forEach(service => {{
                    const card = document.querySelector(`[data-service="${{service.name}}"]`);
                    if (card) {{
                        // Update status badge
                        const statusEl = card.querySelector('.service-status');
                        statusEl.className = `service-status ${{service.status}}`;
                        statusEl.innerHTML = `<span class="status-dot"></span>${{service.status.charAt(0).toUpperCase() + service.status.slice(1)}}`;

                        // Update latency
                        const latencyEl = card.querySelector('.latency-value');
                        if (latencyEl) {{
                            latencyEl.textContent = service.latency_ms ? `${{service.latency_ms}}ms` : 'N/A';
                        }}

                        // Update last check time
                        const timeEl = card.querySelector('.last-check-value');
                        if (timeEl) {{
                            const date = new Date(service.last_check);
                            timeEl.textContent = date.toLocaleTimeString();
                        }}
                    }}
                }});
            }}
        }}

        async function refreshStatus() {{
            const btn = event.target.closest('button');
            const originalContent = btn.innerHTML;
            btn.innerHTML = '<div class="loading-spinner"></div>';
            btn.disabled = true;

            try {{
                const response = await fetch('/api/status');
                const services = await response.json();

                const metricsResponse = await fetch('/api/metrics');
                const metrics = await metricsResponse.json();

                updateDashboard({{ services, metrics }});
                showToast('Status refreshed successfully', 'success');
            }} catch (error) {{
                console.error('Failed to refresh status:', error);
                showToast('Failed to refresh status', 'error');
            }} finally {{
                btn.innerHTML = originalContent;
                btn.disabled = false;
            }}
        }}

        function showToast(message, type = 'success') {{
            const container = document.getElementById('toast-container');
            const toast = document.createElement('div');
            toast.className = `toast ${{type}}`;

            const icon = type === 'success'
                ? '<svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M5 13l4 4L19 7"/></svg>'
                : '<svg class="toast-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M6 18L18 6M6 6l12 12"/></svg>';

            toast.innerHTML = `${{icon}}<span>${{message}}</span>`;
            container.appendChild(toast);

            setTimeout(() => {{
                toast.style.opacity = '0';
                toast.style.transform = 'translateX(100%)';
                setTimeout(() => toast.remove(), 300);
            }}, 4000);
        }}

        function openService(port) {{
            window.open(`http://localhost:${{port}}`, '_blank');
        }}

        // Initialize
        document.addEventListener('DOMContentLoaded', () => {{
            connectSSE();

            // Initial refresh
            setTimeout(() => {{
                fetch('/api/status').then(r => r.json()).then(services => {{
                    fetch('/api/metrics').then(r => r.json()).then(metrics => {{
                        updateDashboard({{ services, metrics }});
                    }});
                }});
            }}, 500);
        }});

        // Cleanup on page unload
        window.addEventListener('beforeunload', () => {{
            if (eventSource) {{
                eventSource.close();
            }}
        }});
    </script>
</body>
</html>"##,
        total = metrics.total_services,
        online = metrics.online_services,
        offline = metrics.offline_services,
        uptime = metrics.uptime_percentage,
        latency = metrics.avg_latency_ms,
        offline_class = if metrics.offline_services == 0 { "positive" } else { "negative" },
        offline_status = if metrics.offline_services == 0 { "All systems go" } else { "Needs attention" },
        service_cards = service_cards,
    )
}

/// Generate HTML for a single service card
fn generate_service_card(service: &ServiceStatus) -> String {
    let latency_display = service
        .latency_ms
        .map(|l| format!("{}ms", l))
        .unwrap_or_else(|| "N/A".to_string());

    let last_check = chrono::DateTime::parse_from_rfc3339(&service.last_check)
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|_| "Unknown".to_string());

    format!(
        r##"<div class="service-card" data-service="{name}" style="--card-color: {color}">
    <div class="service-header">
        <div class="service-icon" style="background: {color}">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="{icon}"/>
            </svg>
        </div>
        <div class="service-info">
            <h3 class="service-name">{display_name}</h3>
            <p class="service-description">{description}</p>
        </div>
        <div class="service-status {status}">
            <span class="status-dot"></span>
            {status_display}
        </div>
    </div>
    <div class="service-meta">
        <div class="service-meta-item">
            <span class="service-meta-label">Port</span>
            <span class="service-meta-value">{port}</span>
        </div>
        <div class="service-meta-item">
            <span class="service-meta-label">Latency</span>
            <span class="service-meta-value latency-value">{latency}</span>
        </div>
        <div class="service-meta-item">
            <span class="service-meta-label">Last Check</span>
            <span class="service-meta-value last-check-value">{last_check}</span>
        </div>
    </div>
    <div class="service-actions">
        <button class="service-btn service-btn-primary" onclick="openService({port})">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
            </svg>
            Open Service
        </button>
        <a href="{docs_url}" class="service-btn service-btn-secondary">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
            </svg>
            Docs
        </a>
    </div>
</div>"##,
        name = service.name,
        display_name = service.display_name,
        description = service.description,
        port = service.port,
        status = service.status,
        status_display = capitalize_first(&service.status),
        latency = latency_display,
        last_check = last_check,
        icon = service.icon,
        color = service.color,
        docs_url = service.docs_url.as_deref().unwrap_or("#"),
    )
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().chain(chars).collect(),
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .pretty()
        .init();

    info!("Starting OpeniBank Portal Dashboard...");

    // Initialize application state
    let state = Arc::new(AppState::new());

    // Initial health check
    update_all_statuses(&state).await;

    // Spawn background health check task
    let health_state = Arc::clone(&state);
    tokio::spawn(async move {
        health_check_task(health_state).await;
    });

    // Build router
    let app = Router::new()
        // Main routes
        .route("/", get(dashboard_handler))
        .route("/health", get(health_handler))
        // API routes
        .route("/api/status", get(api_status_handler))
        .route("/api/metrics", get(api_metrics_handler))
        .route("/api/sse", get(sse_handler))
        // Add middleware
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:9000";
    info!("Portal Dashboard running at http://{}", addr);
    info!("Monitoring services: Playground (8080), Exchange (8888), API (3000), Issuer (8081)");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
