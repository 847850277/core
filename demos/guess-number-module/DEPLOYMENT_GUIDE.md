# 🚀 部署指南

## 概述

本指南提供了将 Calimero 猜数字游戏部署到生产环境的完整步骤。

## 部署架构

```
┌─────────────────────────────────────────────────────────────┐
│                    生产环境架构                               │
├─────────────────────────────────────────────────────────────┤
│  Load Balancer (Nginx)                                     │
│  ├── Web Client (游戏界面)                                  │
│  └── API Server (Axum)                                     │
│      ├── Game Logic (本地处理)                             │
│      └── Calimero Integration                              │
│          └── NEAR Blockchain (数据存储)                    │
├─────────────────────────────────────────────────────────────┤
│  Monitoring Stack                                          │
│  ├── Prometheus (指标收集)                                 │
│  ├── Grafana (数据可视化)                                  │
│  └── Loki (日志聚合)                                       │
└─────────────────────────────────────────────────────────────┘
```

## 环境要求

### 系统要求

- **操作系统**: Linux (Ubuntu 20.04+ 推荐)
- **CPU**: 2+ 核心
- **内存**: 4GB+ RAM
- **存储**: 20GB+ 可用空间
- **网络**: 稳定的互联网连接

### 软件依赖

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# 安装 Docker 和 Docker Compose
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh
sudo curl -L "https://github.com/docker/compose/releases/download/v2.20.0/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# 安装 Nginx
sudo apt update
sudo apt install nginx
```

## 部署步骤

### 1. 代码部署

```bash
# 克隆代码
git clone https://github.com/calimero-network/core.git
cd core/demos/guess-number-module

# 构建生产版本
cargo build --release --bin guess-number-client
cargo build --release --bin guess-number-server
```

### 2. 配置文件

创建生产环境配置文件：

```toml
# config/production.toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[calimero]
node_url = "https://mainnet.calimero.network"
context_id = "guess-number-production"
timeout_seconds = 30
retry_attempts = 3

[database]
max_connections = 20
connection_timeout = 30

[cache]
ttl_seconds = 600
max_size = 10000

[security]
rate_limit_per_minute = 120
enable_cors = true
allowed_origins = ["https://yourdomain.com"]

[monitoring]
metrics_enabled = true
metrics_port = 9090
log_level = "info"
```

### 3. Docker 配置

创建 `Dockerfile`：

```dockerfile
# Dockerfile
FROM rust:1.70 AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin guess-number-server

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/guess-number-server /app/
COPY --from=builder /app/config/ /app/config/

EXPOSE 8080

CMD ["./guess-number-server"]
```

创建 `docker-compose.prod.yml`：

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  guess-number-app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - RUST_LOG=info
      - CONFIG_PATH=/app/config/production.toml
    volumes:
      - ./logs:/app/logs
      - ./config:/app/config:ro
    restart: unless-stopped
    networks:
      - app-network

  nginx:
    image: nginx:alpine
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/ssl:ro
      - ./static:/usr/share/nginx/html:ro
    depends_on:
      - guess-number-app
    restart: unless-stopped
    networks:
      - app-network

  prometheus:
    image: prom/prometheus
    ports:
      - "9090:9090"
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
    restart: unless-stopped
    networks:
      - app-network

  grafana:
    image: grafana/grafana
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin123
    volumes:
      - grafana-data:/var/lib/grafana
      - ./monitoring/grafana:/etc/grafana/provisioning
    restart: unless-stopped
    networks:
      - app-network

volumes:
  prometheus-data:
  grafana-data:

networks:
  app-network:
    driver: bridge
```

### 4. Nginx 配置

创建 `nginx.conf`：

```nginx
events {
    worker_connections 1024;
}

http {
    upstream app_backend {
        server guess-number-app:8080;
    }

    # HTTP to HTTPS 重定向
    server {
        listen 80;
        server_name yourdomain.com www.yourdomain.com;
        return 301 https://$server_name$request_uri;
    }

    # HTTPS 配置
    server {
        listen 443 ssl http2;
        server_name yourdomain.com www.yourdomain.com;

        # SSL 配置
        ssl_certificate /etc/ssl/cert.pem;
        ssl_certificate_key /etc/ssl/key.pem;
        ssl_session_timeout 1d;
        ssl_session_cache shared:SSL:50m;
        ssl_stapling on;
        ssl_stapling_verify on;

        # 安全头
        add_header X-Frame-Options DENY;
        add_header X-Content-Type-Options nosniff;
        add_header X-XSS-Protection "1; mode=block";

        # 静态文件
        location / {
            root /usr/share/nginx/html;
            try_files $uri $uri/ @backend;
        }

        # API 代理
        location /api/ {
            proxy_pass http://app_backend;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
        }

        # WebSocket 支持
        location /ws/ {
            proxy_pass http://app_backend;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
        }

        # 健康检查
        location /health {
            proxy_pass http://app_backend;
            access_log off;
        }

        # 指标端点
        location /metrics {
            proxy_pass http://app_backend;
            allow 127.0.0.1;
            deny all;
        }

        # 静态资源缓存
        location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg)$ {
            expires 1y;
            add_header Cache-Control "public, immutable";
        }
    }
}
```

### 5. SSL 证书配置

```bash
# 使用 Let's Encrypt 获取免费 SSL 证书
sudo apt install certbot python3-certbot-nginx

# 获取证书
sudo certbot --nginx -d yourdomain.com -d www.yourdomain.com

# 设置自动续期
sudo crontab -e
# 添加以下行
0 12 * * * /usr/bin/certbot renew --quiet
```

### 6. 监控配置

创建 `monitoring/prometheus.yml`：

```yaml
# monitoring/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'guess-number-game'
    static_configs:
      - targets: ['guess-number-app:9090']
    metrics_path: '/metrics'
    scrape_interval: 10s

  - job_name: 'node-exporter'
    static_configs:
      - targets: ['localhost:9100']

rule_files:
  - "alert.rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets:
          - alertmanager:9093
```

创建监控告警规则 `monitoring/alert.rules.yml`：

```yaml
# monitoring/alert.rules.yml
groups:
  - name: guess-number-game
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "高错误率检测"
          description: "5分钟内错误率超过 10%"

      - alert: HighResponseTime
        expr: histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m])) > 1
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "响应时间过慢"
          description: "95% 的请求响应时间超过 1 秒"

      - alert: CalimeroConnectionDown
        expr: calimero_connection_status == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Calimero 连接断开"
          description: "与 Calimero 网络的连接已断开"
```

### 7. 部署脚本

创建 `deploy.sh`：

```bash
#!/bin/bash
# deploy.sh

set -e

echo "🚀 开始部署猜数字游戏到生产环境..."

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查环境
check_prerequisites() {
    echo -e "${YELLOW}🔍 检查部署环境...${NC}"
    
    # 检查 Docker
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker 未安装${NC}"
        exit 1
    fi
    
    # 检查 Docker Compose
    if ! command -v docker-compose &> /dev/null; then
        echo -e "${RED}❌ Docker Compose 未安装${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ 环境检查通过${NC}"
}

# 构建应用
build_app() {
    echo -e "${YELLOW}📦 构建应用...${NC}"
    
    cargo build --release --bin guess-number-server
    cargo test --release
    
    echo -e "${GREEN}✅ 应用构建完成${NC}"
}

# 构建 Docker 镜像
build_docker() {
    echo -e "${YELLOW}🐳 构建 Docker 镜像...${NC}"
    
    docker-compose -f docker-compose.prod.yml build
    
    echo -e "${GREEN}✅ Docker 镜像构建完成${NC}"
}

# 部署应用
deploy_app() {
    echo -e "${YELLOW}🌐 部署应用...${NC}"
    
    # 停止旧版本
    docker-compose -f docker-compose.prod.yml down
    
    # 启动新版本
    docker-compose -f docker-compose.prod.yml up -d
    
    echo -e "${GREEN}✅ 应用部署完成${NC}"
}

# 健康检查
health_check() {
    echo -e "${YELLOW}🔍 执行健康检查...${NC}"
    
    # 等待服务启动
    sleep 30
    
    # 检查服务状态
    if curl -f http://localhost/health > /dev/null 2>&1; then
        echo -e "${GREEN}✅ 健康检查通过${NC}"
    else
        echo -e "${RED}❌ 健康检查失败${NC}"
        docker-compose -f docker-compose.prod.yml logs
        exit 1
    fi
}

# 主执行流程
main() {
    check_prerequisites
    build_app
    build_docker
    deploy_app
    health_check
    
    echo -e "${GREEN}🎉 部署完成！${NC}"
    echo "🌐 游戏地址: https://yourdomain.com"
    echo "📊 监控面板: https://yourdomain.com:3000 (admin/admin123)"
    echo "📈 指标查看: https://yourdomain.com:9090"
}

# 执行主函数
main "$@"
```

### 8. 运行部署

```bash
# 设置执行权限
chmod +x deploy.sh

# 运行部署
./deploy.sh
```

## 生产环境优化

### 性能优化

1. **应用级优化**：
```rust
// 在 main.rs 中设置
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() -> eyre::Result<()> {
    // 设置连接池
    let pool = create_connection_pool(20).await?;
    
    // 启用压缩
    let app = Router::new()
        .layer(CompressionLayer::new())
        .layer(CorsLayer::permissive());
    
    Ok(())
}
```

2. **数据库连接优化**：
```toml
# config/production.toml
[database]
max_connections = 50
min_connections = 5
connection_timeout = 30
idle_timeout = 600
```

3. **缓存策略**：
```rust
// 实现多级缓存
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MultiLevelCache {
    l1_cache: Arc<RwLock<LruCache<String, PlayerStats>>>,
    l2_cache: Arc<dyn CacheBackend>,
}
```

### 安全配置

1. **防火墙规则**：
```bash
# 只允许必要端口
sudo ufw allow 22    # SSH
sudo ufw allow 80    # HTTP
sudo ufw allow 443   # HTTPS
sudo ufw enable
```

2. **应用安全**：
```rust
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

let app = Router::new()
    .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1MB limit
    .layer(TimeoutLayer::new(Duration::from_secs(30)))
    .layer(rate_limit_layer());
```

### 监控和告警

1. **日志配置**：
```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new("info"))
    .with(tracing_subscriber::fmt::layer().json())
    .init();
```

2. **指标收集**：
```rust
use prometheus::{Counter, Histogram, register_counter, register_histogram};

lazy_static! {
    static ref HTTP_COUNTER: Counter = register_counter!("http_requests_total", "Total HTTP requests").unwrap();
    static ref HTTP_HISTOGRAM: Histogram = register_histogram!("http_request_duration_seconds", "HTTP request duration").unwrap();
}
```

## 运维管理

### 日常维护

1. **日志管理**：
```bash
# 日志轮转配置
sudo vim /etc/logrotate.d/guess-number-game
```

```
/app/logs/*.log {
    daily
    missingok
    rotate 52
    compress
    delaycompress
    notifempty
    create 0644 app app
}
```

2. **备份策略**：
```bash
#!/bin/bash
# backup.sh

# 备份配置文件
tar -czf "config-backup-$(date +%Y%m%d).tar.gz" config/

# 备份监控数据
docker run --rm -v prometheus-data:/data -v $(pwd):/backup alpine tar czf /backup/prometheus-backup-$(date +%Y%m%d).tar.gz -C /data .
```

### 故障排除

1. **常用调试命令**：
```bash
# 查看服务状态
docker-compose -f docker-compose.prod.yml ps

# 查看日志
docker-compose -f docker-compose.prod.yml logs -f guess-number-app

# 进入容器调试
docker-compose -f docker-compose.prod.yml exec guess-number-app bash

# 检查资源使用
docker stats

# 检查网络连接
netstat -tulpn | grep :8080
```

2. **性能分析**：
```bash
# CPU 和内存使用
top -p $(pgrep guess-number)

# 网络连接数
ss -tuln | grep :8080

# 磁盘 I/O
iostat -x 1
```

### 更新部署

1. **滚动更新脚本**：
```bash
#!/bin/bash
# rolling-update.sh

echo "🔄 开始滚动更新..."

# 拉取最新代码
git pull origin main

# 构建新镜像
docker-compose -f docker-compose.prod.yml build

# 滚动更新
docker-compose -f docker-compose.prod.yml up -d --no-deps guess-number-app

# 健康检查
sleep 30
curl -f http://localhost/health || (echo "更新失败，回滚中..." && git checkout HEAD~1 && ./deploy.sh)

echo "✅ 更新完成"
```

2. **数据库迁移**：
```bash
# 如果需要数据迁移
./guess-number-server migrate --config config/production.toml
```

## 监控面板

### Grafana 仪表板配置

导入预配置的仪表板：

```json
{
  "dashboard": {
    "title": "Guess Number Game Dashboard",
    "panels": [
      {
        "title": "Total Games",
        "type": "stat",
        "targets": [
          {
            "expr": "games_total"
          }
        ]
      },
      {
        "title": "Success Rate",
        "type": "stat",
        "targets": [
          {
            "expr": "rate(successful_stores[5m]) / rate(total_stores[5m]) * 100"
          }
        ]
      },
      {
        "title": "Response Time",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))"
          }
        ]
      }
    ]
  }
}
```

## 扩容指南

### 水平扩容

```yaml
# docker-compose.scale.yml
services:
  guess-number-app:
    # ... 现有配置
    deploy:
      replicas: 3
      
  nginx:
    # 更新 upstream 配置
    volumes:
      - ./nginx-scaled.conf:/etc/nginx/nginx.conf:ro
```

### 负载均衡

```nginx
# nginx-scaled.conf
upstream app_backend {
    server guess-number-app_1:8080 weight=1;
    server guess-number-app_2:8080 weight=1;
    server guess-number-app_3:8080 weight=1;
}
```

---

## 总结

本部署指南涵盖了从开发到生产的完整部署流程。按照这些步骤，您可以成功将 Calimero 猜数字游戏部署到生产环境，并确保系统的稳定性、安全性和可扩展性。

关键要点：
- ✅ 完整的 Docker 容器化部署
- ✅ HTTPS 和安全配置
- ✅ 监控和告警系统
- ✅ 自动化部署脚本
- ✅ 故障排除指南
- ✅ 扩容策略

建议在部署前在测试环境中完整验证所有步骤。
