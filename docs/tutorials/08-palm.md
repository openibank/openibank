# Tutorial 8: Fleet Orchestration with PALM

> **Duration**: 45 minutes
> **Level**: Advanced
> **Prerequisites**: Tutorials 1-7, Understanding of distributed systems

---

## Overview

PALM (Platform for Agent Lifecycle Management) is OpeniBank's fleet orchestration system for deploying and managing AI agent swarms at scale. In this tutorial, you'll learn to:

- Deploy agent fleets with PALM
- Configure deployment strategies
- Monitor fleet health and performance
- Scale agents dynamically
- Implement blue-green deployments

---

## Understanding PALM

```
┌─────────────────────────────────────────────────────────────────┐
│                       PALM Architecture                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│   ┌────────────────────────────────────────────────────────┐    │
│   │                    PALM Control Plane                  │    │
│   │                                                        │    │
│   │   ┌──────────┐  ┌──────────┐  ┌──────────┐             │    │
│   │   │ Registry │  │ Deployer │  │  Health  │             │    │
│   │   │          │  │          │  │ Monitor  │             │    │
│   │   └──────────┘  └──────────┘  └──────────┘             │    │
│   │         │            │              │                  │    │
│   │         └────────────┼──────────────┘                  │    │
│   └────────────────────────────────────────────────────────┘    │
│                          │                                      │
│   ┌──────────────────────▼─────────────────────────────────┐    │
│   │                   Agent Fleet                          │    │
│   │                                                        │    │
│   │   ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐ ┌───────┐    │    │
│   │   │Agent 1│ │Agent 2│ │Agent 3│ │Agent 4│ │Agent N│    │    │
│   │   └───────┘ └───────┘ └───────┘ └───────┘ └───────┘    │    │
│   │                                                        │    │
│   └────────────────────────────────────────────────────────┘    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Step 1: Initialize PALM

### Start PALM Control Plane

```bash
# Start the unified server (includes PALM)
cargo run -p openibank-server

# Access PALM dashboard at http://localhost:8080/palm
```

### Connect via UAL

```bash
# UAL Console
> PALM STATUS

Fleet Status:
  Agents: 0 / 100 (capacity)
  Healthy: 0
  Unhealthy: 0
  Deploying: 0

Resources:
  Total IUSD: $0.00
  Available Capacity: 100 agents
```

---

## Step 2: Deploy Agent Fleets

### Basic Fleet Deployment

```bash
# Deploy 5 buyer agents
> DEPLOY buyer COUNT 5

Deploying 5 buyer agents...
  ✓ buyer_001 - Ready
  ✓ buyer_002 - Ready
  ✓ buyer_003 - Ready
  ✓ buyer_004 - Ready
  ✓ buyer_005 - Ready

Fleet deployed successfully!
```

### Deployment with Configuration

```bash
# Deploy with specific configuration
> DEPLOY seller COUNT 3 WITH {
    "funding": 100000,
    "budget": 50000,
    "service": "Data Analysis",
    "price": 5000,
    "strategy": "conservative"
  }

Deploying 3 seller agents with config...
  ✓ seller_001 - Funded: $100,000 - Service: Data Analysis
  ✓ seller_002 - Funded: $100,000 - Service: Data Analysis
  ✓ seller_003 - Funded: $100,000 - Service: Data Analysis
```

### Rust API Deployment

```rust
use openibank_palm::{PalmClient, DeploymentConfig, AgentSpec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let palm = PalmClient::new("http://localhost:8080");

    // Define agent specification
    let spec = AgentSpec::builder()
        .agent_type("buyer")
        .count(10)
        .funding(100_000)
        .budget(50_000)
        .strategy("momentum")
        .labels([("environment", "production"), ("team", "trading")])
        .build();

    // Deploy with rolling strategy
    let deployment = DeploymentConfig::builder()
        .name("trading-fleet-v1")
        .spec(spec)
        .strategy(DeploymentStrategy::Rolling {
            max_unavailable: 2,
            max_surge: 3,
        })
        .health_check(HealthCheck {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            retries: 3,
        })
        .build();

    // Execute deployment
    let result = palm.deploy(deployment).await?;

    println!("Deployment ID: {}", result.deployment_id);
    println!("Agents deployed: {}", result.agents.len());

    // Wait for all agents to be healthy
    palm.wait_for_healthy(&result.deployment_id, Duration::from_secs(120)).await?;

    Ok(())
}
```

---

## Step 3: Fleet Configuration

### Agent Types

```yaml
# fleet-config.yaml
deployments:
  - name: buyer-fleet
    agent_type: buyer
    count: 20
    config:
      funding: 50000
      budget: 25000
      strategy: aggressive
      risk_tolerance: 0.8
    labels:
      role: buyer
      tier: gold

  - name: seller-fleet
    agent_type: seller
    count: 10
    config:
      funding: 100000
      service: "Market Making"
      price: 10000
      spread: 0.002
    labels:
      role: seller
      tier: platinum

  - name: arbiter-fleet
    agent_type: arbiter
    count: 5
    config:
      escrow_threshold: 10000
      dispute_timeout: 3600
    labels:
      role: arbiter
      trust_level: high
```

### Deploy from Config

```rust
async fn deploy_from_config(palm: &PalmClient) -> Result<(), Box<dyn std::error::Error>> {
    let config = std::fs::read_to_string("fleet-config.yaml")?;
    let fleet_config: FleetConfig = serde_yaml::from_str(&config)?;

    for deployment in fleet_config.deployments {
        println!("Deploying {}...", deployment.name);

        let result = palm.deploy(deployment).await?;

        println!("  Deployed {} agents", result.agents.len());
        for agent in &result.agents {
            println!("    - {} ({})", agent.id, agent.status);
        }
    }

    Ok(())
}
```

---

## Step 4: Monitor Fleet Health

### Health Dashboard

```bash
# UAL Console
> FLEET STATUS

┌──────────────────────────────────────────────────────────────┐
│                      Fleet Overview                          │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  Deployment: trading-fleet-v1                                │
│  Status: HEALTHY                                             │
│                                                              │
│  Agents:                                                     │
│    ✓ Healthy:    35 / 35 (100%)                              │
│    ⚠ Degraded:   0                                           │
│    ✗ Unhealthy:  0                                           │
│                                                              │
│  Performance:                                                │
│    Total PnL:       +$45,230                                 │
│    Avg Latency:     12ms                                     │
│    Orders/min:      1,234                                    │
│    Success Rate:    99.2%                                    │
│                                                              │
│  Resources:                                                  │
│    Total Funding:   $3,500,000                               │
│    Total Budget:    $1,750,000                               │
│    Used Budget:     $823,450 (47%)                           │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Programmatic Health Monitoring

```rust
use openibank_palm::{PalmClient, HealthStatus, AgentMetrics};

async fn monitor_fleet(palm: &PalmClient) -> Result<(), Box<dyn std::error::Error>> {
    let deployment_id = "trading-fleet-v1";

    // Subscribe to health updates
    let mut health_stream = palm.subscribe_health(deployment_id).await?;

    while let Some(update) = health_stream.next().await {
        match update.status {
            HealthStatus::Healthy => {
                println!("✓ {} is healthy", update.agent_id);
            }
            HealthStatus::Degraded { reason } => {
                println!("⚠ {} is degraded: {}", update.agent_id, reason);
                // Maybe reduce workload
                palm.update_agent(&update.agent_id, |config| {
                    config.rate_limit = config.rate_limit / 2;
                }).await?;
            }
            HealthStatus::Unhealthy { reason } => {
                println!("✗ {} is unhealthy: {}", update.agent_id, reason);
                // Restart the agent
                palm.restart_agent(&update.agent_id).await?;
            }
        }
    }

    Ok(())
}
```

### Metrics Collection

```rust
async fn collect_fleet_metrics(palm: &PalmClient, deployment_id: &str) -> Result<FleetMetrics, Box<dyn std::error::Error>> {
    let agents = palm.list_agents(deployment_id).await?;

    let mut total_pnl = dec!(0);
    let mut total_trades = 0u64;
    let mut total_latency_ms = 0u64;
    let mut error_count = 0u64;

    for agent in &agents {
        let metrics = palm.get_agent_metrics(&agent.id).await?;

        total_pnl += metrics.pnl;
        total_trades += metrics.trade_count;
        total_latency_ms += metrics.avg_latency_ms * metrics.trade_count;
        error_count += metrics.error_count;
    }

    let avg_latency = if total_trades > 0 {
        total_latency_ms / total_trades
    } else {
        0
    };

    let success_rate = if total_trades > 0 {
        (total_trades - error_count) as f64 / total_trades as f64 * 100.0
    } else {
        0.0
    };

    Ok(FleetMetrics {
        agent_count: agents.len(),
        total_pnl,
        total_trades,
        avg_latency_ms: avg_latency,
        success_rate,
        timestamp: Utc::now(),
    })
}
```

---

## Step 5: Scaling Operations

### Manual Scaling

```bash
# Scale up
> SCALE trading-fleet-v1 TO 50

Scaling trading-fleet-v1 from 35 to 50 agents...
  Deploying 15 new agents...
  ✓ buyer_036 - Ready
  ✓ buyer_037 - Ready
  ...
  ✓ buyer_050 - Ready

Fleet scaled successfully!

# Scale down
> SCALE trading-fleet-v1 TO 20

Scaling trading-fleet-v1 from 50 to 20 agents...
  Gracefully terminating 30 agents...
  ⏳ buyer_050 - Closing positions...
  ✓ buyer_050 - Terminated
  ...
```

### Auto-Scaling

```rust
use openibank_palm::{AutoScaler, ScalingPolicy, ScalingMetric};

async fn setup_autoscaling(palm: &PalmClient, deployment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let policy = ScalingPolicy::builder()
        .min_replicas(10)
        .max_replicas(100)
        .target_metrics([
            ScalingMetric::CpuUtilization { target: 70.0 },
            ScalingMetric::OrdersPerSecond { target: 100.0 },
            ScalingMetric::LatencyMs { target: 50.0 },
        ])
        .scale_up(ScalingBehavior {
            stabilization_window: Duration::from_secs(60),
            policies: vec![
                ScalingStep { type_: "Pods", value: 4, period: Duration::from_secs(60) },
                ScalingStep { type_: "Percent", value: 100, period: Duration::from_secs(60) },
            ],
            select_policy: "Max",
        })
        .scale_down(ScalingBehavior {
            stabilization_window: Duration::from_secs(300),
            policies: vec![
                ScalingStep { type_: "Pods", value: 2, period: Duration::from_secs(60) },
            ],
            select_policy: "Min",
        })
        .build();

    palm.set_autoscaling(deployment_id, policy).await?;

    println!("Autoscaling enabled for {}", deployment_id);

    Ok(())
}
```

### Load-Based Scaling

```rust
async fn reactive_scaling(palm: &PalmClient, deployment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut metrics_stream = palm.subscribe_metrics(deployment_id).await?;

    while let Some(metrics) = metrics_stream.next().await {
        let current_count = palm.get_agent_count(deployment_id).await?;

        // Scale based on order backlog
        if metrics.order_backlog > 1000 && current_count < 100 {
            let new_count = (current_count as f64 * 1.5).ceil() as u32;
            palm.scale(deployment_id, new_count).await?;
            println!("Scaled up to {} due to high backlog", new_count);
        }

        // Scale based on low utilization
        if metrics.cpu_utilization < 20.0 && current_count > 10 {
            let new_count = (current_count as f64 * 0.75).floor() as u32;
            palm.scale(deployment_id, new_count.max(10)).await?;
            println!("Scaled down to {} due to low utilization", new_count);
        }
    }

    Ok(())
}
```

---

## Step 6: Deployment Strategies

### Rolling Deployment

```rust
async fn rolling_update(palm: &PalmClient, deployment_id: &str, new_spec: AgentSpec) -> Result<(), Box<dyn std::error::Error>> {
    let strategy = UpdateStrategy::Rolling {
        max_unavailable: 2,  // At most 2 agents down at once
        max_surge: 3,        // Can temporarily have 3 extra
        ready_seconds: 30,   // Wait 30s after healthy before continuing
    };

    palm.update_deployment(deployment_id, new_spec, strategy).await?;

    // Monitor rollout progress
    let mut progress = palm.watch_rollout(deployment_id).await?;

    while let Some(status) = progress.next().await {
        println!(
            "Rollout progress: {}/{} updated, {}/{} available",
            status.updated,
            status.total,
            status.available,
            status.total
        );

        if status.complete {
            println!("Rollout complete!");
            break;
        }
    }

    Ok(())
}
```

### Blue-Green Deployment

```rust
async fn blue_green_deployment(palm: &PalmClient, new_spec: AgentSpec) -> Result<(), Box<dyn std::error::Error>> {
    // Current deployment is "blue"
    let blue_id = "trading-fleet-blue";

    // Deploy new version as "green"
    let green_config = DeploymentConfig::builder()
        .name("trading-fleet-green")
        .spec(new_spec)
        .build();

    let green = palm.deploy(green_config).await?;
    println!("Green deployment created: {}", green.deployment_id);

    // Wait for green to be fully healthy
    palm.wait_for_healthy(&green.deployment_id, Duration::from_secs(300)).await?;
    println!("Green deployment is healthy");

    // Run smoke tests on green
    let test_results = run_smoke_tests(&green.deployment_id).await?;
    if !test_results.all_passed() {
        println!("Smoke tests failed, rolling back");
        palm.delete_deployment(&green.deployment_id).await?;
        return Err("Deployment failed smoke tests".into());
    }

    // Switch traffic to green
    println!("Switching traffic to green...");
    palm.switch_traffic("trading-fleet", &green.deployment_id).await?;

    // Keep blue running briefly for quick rollback
    tokio::time::sleep(Duration::from_secs(300)).await;

    // If no issues, delete blue
    println!("Deleting blue deployment...");
    palm.delete_deployment(blue_id).await?;

    // Rename green to blue for next deployment
    palm.rename_deployment(&green.deployment_id, "trading-fleet-blue").await?;

    println!("Blue-green deployment complete!");

    Ok(())
}
```

### Canary Deployment

```rust
async fn canary_deployment(palm: &PalmClient, new_spec: AgentSpec) -> Result<(), Box<dyn std::error::Error>> {
    let main_deployment = "trading-fleet-main";

    // Deploy canary with 10% of traffic
    let canary_config = DeploymentConfig::builder()
        .name("trading-fleet-canary")
        .spec(new_spec.with_count(5))  // Small canary
        .traffic_weight(10)  // 10% of traffic
        .build();

    let canary = palm.deploy(canary_config).await?;

    // Monitor canary for issues
    let monitor_duration = Duration::from_secs(1800);  // 30 minutes
    let start = Instant::now();

    while start.elapsed() < monitor_duration {
        let canary_metrics = palm.get_deployment_metrics(&canary.deployment_id).await?;
        let main_metrics = palm.get_deployment_metrics(main_deployment).await?;

        // Compare error rates
        if canary_metrics.error_rate > main_metrics.error_rate * 1.5 {
            println!("Canary error rate too high, rolling back");
            palm.delete_deployment(&canary.deployment_id).await?;
            return Err("Canary deployment failed".into());
        }

        // Compare latency
        if canary_metrics.p99_latency > main_metrics.p99_latency * 1.2 {
            println!("Canary latency too high, rolling back");
            palm.delete_deployment(&canary.deployment_id).await?;
            return Err("Canary deployment failed".into());
        }

        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    // Canary passed, promote to main
    println!("Canary successful, promoting...");

    // Gradually shift traffic
    for weight in [25, 50, 75, 100] {
        palm.set_traffic_weight(&canary.deployment_id, weight).await?;
        println!("Traffic weight: {}%", weight);
        tokio::time::sleep(Duration::from_secs(60)).await;
    }

    // Delete old deployment
    palm.delete_deployment(main_deployment).await?;
    palm.rename_deployment(&canary.deployment_id, main_deployment).await?;

    println!("Canary deployment promoted!");

    Ok(())
}
```

---

## Step 7: Fleet Operations

### Graceful Shutdown

```rust
async fn graceful_shutdown(palm: &PalmClient, deployment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Initiating graceful shutdown...");

    // Stop accepting new work
    palm.drain_deployment(deployment_id).await?;

    // Wait for in-flight work to complete
    let agents = palm.list_agents(deployment_id).await?;

    for agent in &agents {
        println!("Waiting for {} to complete...", agent.id);

        // Wait for open positions to close
        while palm.get_agent_open_positions(&agent.id).await? > 0 {
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        // Wait for pending orders to fill or cancel
        palm.cancel_agent_orders(&agent.id).await?;

        println!("  {} ready for shutdown", agent.id);
    }

    // Now terminate
    palm.delete_deployment(deployment_id).await?;

    println!("Shutdown complete");

    Ok(())
}
```

### Fleet Maintenance

```rust
async fn rolling_maintenance(palm: &PalmClient, deployment_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let agents = palm.list_agents(deployment_id).await?;

    for chunk in agents.chunks(5) {
        println!("Maintaining {} agents...", chunk.len());

        // Drain chunk
        for agent in chunk {
            palm.drain_agent(&agent.id).await?;
        }

        // Wait for drain
        tokio::time::sleep(Duration::from_secs(30)).await;

        // Perform maintenance (restart with new config)
        for agent in chunk {
            palm.restart_agent(&agent.id).await?;
        }

        // Wait for healthy
        for agent in chunk {
            palm.wait_for_agent_healthy(&agent.id, Duration::from_secs(60)).await?;
        }

        println!("Chunk complete");
    }

    println!("Maintenance complete");

    Ok(())
}
```

---

## Complete Fleet Management Example

```rust
use openibank_palm::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let palm = PalmClient::new("http://localhost:8080");

    // Deploy initial fleet
    let spec = AgentSpec::builder()
        .agent_type("buyer")
        .count(20)
        .funding(100_000)
        .budget(50_000)
        .strategy("momentum")
        .build();

    let deployment = palm.deploy(
        DeploymentConfig::builder()
            .name("production-fleet")
            .spec(spec)
            .strategy(DeploymentStrategy::Rolling {
                max_unavailable: 2,
                max_surge: 3,
            })
            .build()
    ).await?;

    println!("Deployed: {}", deployment.deployment_id);

    // Setup autoscaling
    palm.set_autoscaling(&deployment.deployment_id, ScalingPolicy::builder()
        .min_replicas(10)
        .max_replicas(50)
        .target_metrics([
            ScalingMetric::OrdersPerSecond { target: 100.0 },
        ])
        .build()
    ).await?;

    // Monitor continuously
    tokio::spawn({
        let palm = palm.clone();
        let id = deployment.deployment_id.clone();
        async move {
            let mut metrics = palm.subscribe_metrics(&id).await.unwrap();
            while let Some(m) = metrics.next().await {
                println!("PnL: ${:.2}, Trades: {}, Latency: {}ms",
                    m.pnl, m.trades, m.latency);
            }
        }
    });

    // Run until interrupted
    tokio::signal::ctrl_c().await?;

    // Graceful shutdown
    graceful_shutdown(&palm, &deployment.deployment_id).await?;

    Ok(())
}
```

---

## Troubleshooting

| Issue | Cause | Solution |
|-------|-------|----------|
| `DEPLOYMENT_FAILED` | Config error | Check agent spec and resources |
| `INSUFFICIENT_CAPACITY` | No resources | Scale down or add capacity |
| `HEALTH_CHECK_FAILED` | Agent not responding | Check agent logs, increase timeout |
| `SCALING_STUCK` | Rollout paused | Check events, may need manual intervention |

---

## Next Steps

- [Tutorial 9: Multi-Agent Coordination](./09-multi-agent.md)
- [Tutorial 10: Security & Compliance](./10-security.md)
- [PALM API Reference](../api/README.md#palm)
