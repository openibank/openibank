# OpeniBank Production Deployment Guide

> Deploying OpeniBank to Production

This guide covers deploying OpeniBank services to production environments, including architecture recommendations, configuration, monitoring, and security best practices.

---

## Deployment Architecture

### Recommended Production Setup

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Load Balancer (HTTPS)                          │
│                              (Nginx / Cloudflare / AWS ALB)                 │
└─────────────────────────────────────────────────────────────────────────────┘
                    │                       │                       │
          ┌─────────▼─────────┐   ┌─────────▼─────────┐   ┌─────────▼─────────┐
          │   API Server      │   │   API Server      │   │   API Server      │
          │   (Replica 1)     │   │   (Replica 2)     │   │   (Replica 3)     │
          │   Port 3000       │   │   Port 3000       │   │   Port 3000       │
          └─────────┬─────────┘   └─────────┬─────────┘   └─────────┬─────────┘
                    │                       │                       │
          ┌─────────▼─────────────────────────────────────────────────────────┐
          │                        Redis Cluster                               │
          │                    (Session / Rate Limit / Cache)                  │
          └─────────┬─────────────────────────────────────────────────────────┘
                    │
          ┌─────────▼─────────────────────────────────────────────────────────┐
          │                    PostgreSQL (Primary + Replicas)                 │
          │                         (RDS / Cloud SQL)                          │
          └───────────────────────────────────────────────────────────────────┘
```

### Service Components

| Service | Port | Replicas | Resources |
|---------|------|----------|-----------|
| openibank-api-server | 3000 | 3+ | 2 CPU, 4GB RAM |
| resonancex-server | 8080 | 2+ | 4 CPU, 8GB RAM |
| openibank-playground | 8080 | 1-2 | 1 CPU, 2GB RAM |
| PostgreSQL | 5432 | Primary + 2 replicas | 4 CPU, 16GB RAM |
| Redis | 6379 | 3-node cluster | 2 CPU, 4GB RAM |

---

## Prerequisites

### Infrastructure Requirements

- **Kubernetes cluster** (EKS, GKE, or self-managed) OR Docker Swarm
- **PostgreSQL 14+** (managed preferred: RDS, Cloud SQL)
- **Redis 7+** (managed preferred: ElastiCache, MemoryStore)
- **Domain with SSL certificate**
- **Container registry** (ECR, GCR, Docker Hub)

### Build Requirements

- Rust 1.75+
- Docker 24+
- kubectl or docker-compose

---

## Building Docker Images

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release -p openibank-api-server

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/openibank-api-server /usr/local/bin/

EXPOSE 3000

CMD ["openibank-api-server"]
```

### Build and Push

```bash
# Build images
docker build -t openibank/api-server:v1.0.0 -f services/openibank-api-server/Dockerfile .
docker build -t openibank/resonancex-server:v1.0.0 -f services/resonancex-server/Dockerfile .
docker build -t openibank/playground:v1.0.0 -f services/openibank-playground/Dockerfile .

# Push to registry
docker push openibank/api-server:v1.0.0
docker push openibank/resonancex-server:v1.0.0
docker push openibank/playground:v1.0.0
```

---

## Configuration

### Environment Variables

```bash
# Database
DATABASE_URL=postgres://user:pass@host:5432/openibank
DATABASE_POOL_SIZE=20
DATABASE_TIMEOUT=30

# Redis
REDIS_URL=redis://host:6379
REDIS_CLUSTER_NODES=node1:6379,node2:6379,node3:6379

# API Server
OPENIBANK_HOST=0.0.0.0
OPENIBANK_PORT=3000
OPENIBANK_WORKERS=4

# Authentication
JWT_SECRET=your-256-bit-secret
JWT_ACCESS_EXPIRY=3600
JWT_REFRESH_EXPIRY=2592000
API_KEY_SALT=your-salt

# LLM (for agent features)
OPENIBANK_LLM_PROVIDER=openai
OPENAI_API_KEY=sk-xxx
OPENIBANK_OLLAMA_URL=http://ollama:11434

# Issuer
OPENIBANK_ISSUER_KEYPAIR=/secrets/issuer-keypair.json
OPENIBANK_ISSUER_RESERVE_CAP=100000000000

# Observability
RUST_LOG=info,openibank=debug
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
METRICS_PORT=9090
```

### Configuration File (`config.toml`)

```toml
[server]
host = "0.0.0.0"
port = 3000
workers = 4
request_timeout = 30

[database]
url = "${DATABASE_URL}"
pool_size = 20
connect_timeout = 10
idle_timeout = 600

[redis]
url = "${REDIS_URL}"
pool_size = 10

[auth]
jwt_secret = "${JWT_SECRET}"
access_token_ttl = 3600
refresh_token_ttl = 2592000
bcrypt_cost = 12

[rate_limit]
requests_per_minute = 1200
orders_per_second = 10
orders_per_day = 200000

[trading]
enable_websocket = true
websocket_ping_interval = 30
max_open_orders = 100
min_notional = 10.0

[issuer]
reserve_cap = 100000000000
max_single_mint = 10000000
keypair_path = "/secrets/issuer-keypair.json"

[metrics]
enabled = true
port = 9090
path = "/metrics"
```

---

## Kubernetes Deployment

### Namespace and Secrets

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: openibank

---
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: openibank-secrets
  namespace: openibank
type: Opaque
stringData:
  DATABASE_URL: "postgres://user:pass@postgres:5432/openibank"
  REDIS_URL: "redis://redis:6379"
  JWT_SECRET: "your-256-bit-secret"
  OPENAI_API_KEY: "sk-xxx"
```

### API Server Deployment

```yaml
# api-server-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: api-server
  namespace: openibank
spec:
  replicas: 3
  selector:
    matchLabels:
      app: api-server
  template:
    metadata:
      labels:
        app: api-server
    spec:
      containers:
        - name: api-server
          image: openibank/api-server:v1.0.0
          ports:
            - containerPort: 3000
            - containerPort: 9090
          envFrom:
            - secretRef:
                name: openibank-secrets
          env:
            - name: RUST_LOG
              value: "info,openibank=debug"
          resources:
            requests:
              cpu: "1"
              memory: "2Gi"
            limits:
              cpu: "2"
              memory: "4Gi"
          livenessProbe:
            httpGet:
              path: /api/v1/ping
              port: 3000
            initialDelaySeconds: 10
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /api/v1/ping
              port: 3000
            initialDelaySeconds: 5
            periodSeconds: 5
---
apiVersion: v1
kind: Service
metadata:
  name: api-server
  namespace: openibank
spec:
  selector:
    app: api-server
  ports:
    - name: http
      port: 3000
      targetPort: 3000
    - name: metrics
      port: 9090
      targetPort: 9090
```

### Ingress with TLS

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: openibank-ingress
  namespace: openibank
  annotations:
    kubernetes.io/ingress.class: nginx
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/rate-limit: "100"
    nginx.ingress.kubernetes.io/rate-limit-window: "1m"
spec:
  tls:
    - hosts:
        - api.openibank.com
      secretName: openibank-tls
  rules:
    - host: api.openibank.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: api-server
                port:
                  number: 3000
```

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: api-server-hpa
  namespace: openibank
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: api-server
  minReplicas: 3
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
    - type: Resource
      resource:
        name: memory
        target:
          type: Utilization
          averageUtilization: 80
```

---

## Docker Compose (Single Server)

```yaml
# docker-compose.yml
version: '3.8'

services:
  api-server:
    image: openibank/api-server:latest
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgres://openibank:password@postgres:5432/openibank
      REDIS_URL: redis://redis:6379
      JWT_SECRET: ${JWT_SECRET}
    depends_on:
      - postgres
      - redis
    restart: always

  resonancex:
    image: openibank/resonancex-server:latest
    ports:
      - "8080:8080"
    environment:
      DATABASE_URL: postgres://openibank:password@postgres:5432/openibank
      REDIS_URL: redis://redis:6379
    depends_on:
      - postgres
      - redis
    restart: always

  postgres:
    image: postgres:16
    environment:
      POSTGRES_USER: openibank
      POSTGRES_PASSWORD: password
      POSTGRES_DB: openibank
    volumes:
      - postgres_data:/var/lib/postgresql/data
    restart: always

  redis:
    image: redis:7-alpine
    volumes:
      - redis_data:/data
    restart: always

volumes:
  postgres_data:
  redis_data:
```

---

## Database Migrations

```bash
# Run migrations
cargo run -p openibank-db -- migrate

# Or with sqlx-cli
sqlx migrate run --database-url $DATABASE_URL
```

### Migration Files

Located in `crates/openibank-db/migrations/`:

```
migrations/
├── 20250101000001_create_users.sql
├── 20250101000002_create_wallets.sql
├── 20250101000003_create_orders.sql
├── 20250101000004_create_trades.sql
├── 20250101000005_create_deposits.sql
└── ...
```

---

## Monitoring & Observability

### Prometheus Metrics

The API server exposes metrics at `/metrics`:

```
# Request metrics
http_requests_total{method="POST",path="/api/v1/order",status="200"}
http_request_duration_seconds{method="GET",path="/api/v1/depth"}

# Trading metrics
orders_created_total{side="buy",type="limit"}
orders_filled_total{side="sell"}
trades_executed_total{symbol="BTCUSDT"}

# System metrics
db_pool_connections_active
redis_pool_connections_active
```

### Grafana Dashboard

Import the provided dashboard: `docs/deployment/grafana-dashboard.json`

### Alerting Rules

```yaml
# prometheus-rules.yaml
groups:
  - name: openibank
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: High error rate detected

      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: High latency detected

      - alert: DatabaseConnectionExhausted
        expr: db_pool_connections_active / db_pool_connections_max > 0.9
        for: 2m
        labels:
          severity: critical
        annotations:
          summary: Database connection pool near exhaustion
```

---

## Security Best Practices

### Network Security

1. **Internal Services**: Keep PostgreSQL and Redis internal (no public access)
2. **TLS Everywhere**: Use TLS for all external connections
3. **Network Policies**: Restrict pod-to-pod communication

```yaml
# network-policy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: api-server-policy
  namespace: openibank
spec:
  podSelector:
    matchLabels:
      app: api-server
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              name: ingress-nginx
      ports:
        - port: 3000
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: postgres
      ports:
        - port: 5432
    - to:
        - podSelector:
            matchLabels:
              app: redis
      ports:
        - port: 6379
```

### Secret Management

- Use Kubernetes Secrets or external secret managers (Vault, AWS Secrets Manager)
- Rotate JWT secrets regularly
- Never log sensitive data

### Rate Limiting

Configure at multiple levels:
1. **Ingress/Load Balancer**: Global rate limits
2. **API Server**: Per-user/API-key limits
3. **Redis**: Distributed rate limiting

---

## Backup & Recovery

### Database Backups

```bash
# Automated daily backups
0 2 * * * pg_dump $DATABASE_URL | gzip > /backups/openibank-$(date +%Y%m%d).sql.gz

# Retain 30 days
find /backups -name "openibank-*.sql.gz" -mtime +30 -delete
```

### Recovery Procedure

```bash
# Stop services
kubectl scale deployment api-server --replicas=0

# Restore database
gunzip -c /backups/openibank-20250208.sql.gz | psql $DATABASE_URL

# Run migrations
cargo run -p openibank-db -- migrate

# Restart services
kubectl scale deployment api-server --replicas=3
```

---

## Health Checks

### Endpoints

| Endpoint | Purpose |
|----------|---------|
| `/api/v1/ping` | Basic liveness |
| `/api/v1/time` | Server time |
| `/health` | Detailed health status |

### Health Response

```json
{
  "status": "healthy",
  "version": "1.0.0",
  "uptime": 86400,
  "checks": {
    "database": "ok",
    "redis": "ok",
    "issuer": "ok"
  }
}
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| 503 Service Unavailable | Pods not ready | Check pod logs, readiness probes |
| High latency | Database slow queries | Add indexes, check query plans |
| Connection refused | Network policy | Check network policies |
| JWT errors | Clock skew | Sync NTP across nodes |

### Debug Commands

```bash
# Pod logs
kubectl logs -f deployment/api-server -n openibank

# Database connection
kubectl exec -it deployment/api-server -- psql $DATABASE_URL

# Redis check
kubectl exec -it deployment/api-server -- redis-cli -u $REDIS_URL ping

# Resource usage
kubectl top pods -n openibank
```

---

## Support

For production support:
- **Email**: support@openibank.com
- **Discord**: [discord.gg/openibank](https://discord.gg/openibank)
- **GitHub Issues**: [github.com/openibank/openibank/issues](https://github.com/openibank/openibank/issues)
