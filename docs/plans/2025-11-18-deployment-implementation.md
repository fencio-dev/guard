# Production Deployment Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Containerize and deploy three separate systems (MCP Gateway, AI Security Stack, UI Console) for production use.

**Architecture:** Three independent docker-compose deployments with centralized configuration in `deployment/` directory. Supabase for authentication, filesystem volumes for persistence, supervisord for multi-process containers.

**Tech Stack:** Docker, docker-compose, Node.js, Python, Rust, Nginx, Supabase, supervisord

---

## Prerequisites

Before starting:
- Docker and docker-compose installed
- Supabase project created with Google OAuth enabled
- Gemini API key obtained
- Access to the mgmt-plane repository

---

## Phase 1: Deployment Infrastructure (Week 1)

### Task 1: Create Deployment Directory Structure

**Files:**
- Create: `deployment/gateway/.gitkeep`
- Create: `deployment/security-stack/.gitkeep`
- Create: `deployment/ui/.gitkeep`

**Step 1: Create deployment directories**

```bash
mkdir -p deployment/gateway
mkdir -p deployment/security-stack
mkdir -p deployment/ui
touch deployment/gateway/.gitkeep
touch deployment/security-stack/.gitkeep
touch deployment/ui/.gitkeep
```

**Step 2: Verify structure**

Run: `tree deployment -L 2`
Expected: Three subdirectories visible

**Step 3: Commit**

```bash
git add deployment/
git commit -m "feat(deployment): create deployment directory structure"
```

---

### Task 2: MCP Gateway Dockerfile

**Files:**
- Create: `deployment/gateway/Dockerfile`
- Create: `deployment/gateway/.dockerignore`

**Step 1: Create .dockerignore**

File: `deployment/gateway/.dockerignore`

```
node_modules
dist
.env
*.log
test-generated
tests
workspace
generated
.git
```

**Step 2: Create Dockerfile**

File: `deployment/gateway/Dockerfile`

```dockerfile
# Build stage
FROM node:18-alpine AS builder

WORKDIR /build

# Copy package files
COPY ../../mcp-gateway/package*.json ./

# Install dependencies
RUN npm ci --only=production

# Copy source code
COPY ../../mcp-gateway/ ./

# Build TypeScript
RUN npm run build

# Production stage
FROM node:18-alpine

WORKDIR /app

# Install curl for health checks
RUN apk add --no-cache curl

# Copy built artifacts and dependencies
COPY --from=builder /build/dist ./dist
COPY --from=builder /build/node_modules ./node_modules
COPY --from=builder /build/package*.json ./

# Create directories for runtime data
RUN mkdir -p /app/tenants /app/workspace

# Expose HTTP port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=40s \
  CMD curl -f http://localhost:3000/health || exit 1

# Run the HTTP server
CMD ["node", "dist/http-server.js"]
```

**Step 3: Test build**

Run: `cd deployment/gateway && docker build -t mcp-gateway:test -f Dockerfile ../..`
Expected: Build succeeds (may fail due to missing /health endpoint - acceptable for now)

**Step 4: Commit**

```bash
git add deployment/gateway/Dockerfile deployment/gateway/.dockerignore
git commit -m "feat(deployment): add MCP Gateway Dockerfile"
```

---

### Task 3: MCP Gateway docker-compose.yml

**Files:**
- Create: `deployment/gateway/docker-compose.yml`
- Create: `deployment/gateway/.env.example`

**Step 1: Create .env.example**

File: `deployment/gateway/.env.example`

```bash
# Gemini API Key (required for intelligence layer)
GEMINI_API_KEY=your-gemini-api-key-here

# Gateway Configuration
MCP_GATEWAY_HTTP_PORT=3000
MCP_GATEWAY_TENANTS_ROOT=/app/tenants
MCP_GATEWAY_CHROMA_URL=http://chromadb:8001

# Intelligence Layer
MCP_GATEWAY_INTELLIGENCE_ENABLED=true
MCP_GATEWAY_SUMMARY_THRESHOLD_BYTES=2048
MCP_GATEWAY_INTELLIGENCE_TTL_HOURS=3
```

**Step 2: Create docker-compose.yml**

File: `deployment/gateway/docker-compose.yml`

```yaml
version: '3.8'

services:
  mcp-gateway-http:
    build:
      context: ../..
      dockerfile: deployment/gateway/Dockerfile
    ports:
      - "3000:3000"
    environment:
      - GEMINI_API_KEY=${GEMINI_API_KEY}
      - MCP_GATEWAY_HTTP_PORT=${MCP_GATEWAY_HTTP_PORT:-3000}
      - MCP_GATEWAY_TENANTS_ROOT=${MCP_GATEWAY_TENANTS_ROOT:-/app/tenants}
      - MCP_GATEWAY_CHROMA_URL=${MCP_GATEWAY_CHROMA_URL:-http://chromadb:8001}
      - MCP_GATEWAY_INTELLIGENCE_ENABLED=${MCP_GATEWAY_INTELLIGENCE_ENABLED:-true}
      - MCP_GATEWAY_SUMMARY_THRESHOLD_BYTES=${MCP_GATEWAY_SUMMARY_THRESHOLD_BYTES:-2048}
      - MCP_GATEWAY_INTELLIGENCE_TTL_HOURS=${MCP_GATEWAY_INTELLIGENCE_TTL_HOURS:-3}
    volumes:
      - gateway-tenants:/app/tenants
      - gateway-workspace:/app/workspace
    depends_on:
      chromadb:
        condition: service_healthy
    restart: unless-stopped

  chromadb:
    image: chromadb/chroma:latest
    ports:
      - "8001:8000"
    volumes:
      - gateway-chromadb:/chroma/chroma
    environment:
      - IS_PERSISTENT=TRUE
      - ANONYMIZED_TELEMETRY=FALSE
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/v1/heartbeat"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 20s
    restart: unless-stopped

volumes:
  gateway-tenants:
    driver: local
  gateway-workspace:
    driver: local
  gateway-chromadb:
    driver: local
```

**Step 3: Test docker-compose**

Run: `cd deployment/gateway && cp .env.example .env && docker-compose config`
Expected: Valid docker-compose configuration printed

**Step 4: Commit**

```bash
git add deployment/gateway/docker-compose.yml deployment/gateway/.env.example
git commit -m "feat(deployment): add MCP Gateway docker-compose configuration"
```

---

### Task 4: AI Security Stack Dockerfile

**Files:**
- Create: `deployment/security-stack/Dockerfile`
- Create: `deployment/security-stack/.dockerignore`

**Step 1: Create .dockerignore**

File: `deployment/security-stack/.dockerignore`

```
__pycache__
*.pyc
.pytest_cache
.env
*.log
target
.git
tests
```

**Step 2: Create multi-stage Dockerfile**

File: `deployment/security-stack/Dockerfile`

```dockerfile
# Rust build stage
FROM rust:1.75-slim AS rust-builder

WORKDIR /rust-build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust workspace
COPY tupl_data_plane/ ./tupl_data_plane/
COPY semantic-sandbox/ ./semantic-sandbox/

# Build Data Plane (bridge-server)
WORKDIR /rust-build/tupl_data_plane
RUN cargo build --release --bin bridge-server

# Build semantic sandbox library
WORKDIR /rust-build/semantic-sandbox
RUN cargo build --release --lib

# Python build stage
FROM python:3.12-slim AS python-builder

WORKDIR /python-build

# Install system dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

# Copy Python projects
COPY management-plane/ ./management-plane/
COPY policy_control_plane/ ./policy_control_plane/

# Install Management Plane dependencies
WORKDIR /python-build/management-plane
RUN pip install --no-cache-dir -e .

# Install Control Plane dependencies
WORKDIR /python-build/policy_control_plane
RUN pip install --no-cache-dir -r requirements.txt

# Final runtime stage
FROM python:3.12-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    curl \
    supervisor \
    && rm -rf /var/lib/apt/lists/*

# Copy Rust binaries
COPY --from=rust-builder /rust-build/tupl_data_plane/target/release/bridge-server /app/bridge-server
COPY --from=rust-builder /rust-build/semantic-sandbox/target/release/libsemantic_sandbox.so /app/libsemantic_sandbox.so

# Copy Python applications
COPY --from=python-builder /python-build/management-plane /app/management-plane
COPY --from=python-builder /python-build/policy_control_plane /app/policy_control_plane
COPY --from=python-builder /usr/local/lib/python3.12/site-packages /usr/local/lib/python3.12/site-packages

# Copy supervisord configuration
COPY deployment/security-stack/supervisord.conf /etc/supervisor/conf.d/supervisord.conf

# Create data directories
RUN mkdir -p /app/data /root/.cache/huggingface

# Expose ports
EXPOSE 8000 8001 50051

# Set library path for FFI
ENV LD_LIBRARY_PATH=/app:$LD_LIBRARY_PATH

# Health check (Management Plane)
HEALTHCHECK --interval=30s --timeout=10s --retries=3 --start-period=60s \
  CMD curl -f http://localhost:8000/health || exit 1

# Run supervisord
CMD ["/usr/bin/supervisord", "-c", "/etc/supervisor/conf.d/supervisord.conf"]
```

**Step 3: Test build**

Run: `cd deployment/security-stack && docker build -t ai-security-stack:test -f Dockerfile ../..`
Expected: Build succeeds (may be slow due to model downloads)

**Step 4: Commit**

```bash
git add deployment/security-stack/Dockerfile deployment/security-stack/.dockerignore
git commit -m "feat(deployment): add AI Security Stack Dockerfile"
```

---

### Task 5: AI Security Stack supervisord Configuration

**Files:**
- Create: `deployment/security-stack/supervisord.conf`

**Step 1: Create supervisord.conf**

File: `deployment/security-stack/supervisord.conf`

```ini
[supervisord]
nodaemon=true
logfile=/dev/stdout
logfile_maxbytes=0
loglevel=info

[program:data-plane]
command=/app/bridge-server
directory=/app
priority=1
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
environment=RUST_LOG="info"

[program:management-plane]
command=uvicorn app.main:app --host 0.0.0.0 --port 8000
directory=/app/management-plane
priority=2
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
environment=GOOGLE_API_KEY="%(ENV_GOOGLE_API_KEY)s",DATA_PLANE_GRPC_URL="localhost:50051",LD_LIBRARY_PATH="/app"

[program:control-plane]
command=python server.py
directory=/app/policy_control_plane
priority=3
autostart=true
autorestart=true
stdout_logfile=/dev/stdout
stdout_logfile_maxbytes=0
stderr_logfile=/dev/stderr
stderr_logfile_maxbytes=0
environment=MANAGEMENT_PLANE_URL="http://localhost:8000"
```

**Step 2: Commit**

```bash
git add deployment/security-stack/supervisord.conf
git commit -m "feat(deployment): add supervisord configuration for multi-process container"
```

---

### Task 6: AI Security Stack docker-compose.yml

**Files:**
- Create: `deployment/security-stack/docker-compose.yml`
- Create: `deployment/security-stack/.env.example`

**Step 1: Create .env.example**

File: `deployment/security-stack/.env.example`

```bash
# Google API Key (required for LLM anchor generation)
GOOGLE_API_KEY=your-google-api-key-here

# Supabase Configuration (for JWT validation)
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_JWT_SECRET=your-jwt-secret-here

# Internal Service URLs (container defaults)
DATA_PLANE_GRPC_URL=localhost:50051
MANAGEMENT_PLANE_URL=http://localhost:8000
```

**Step 2: Create docker-compose.yml**

File: `deployment/security-stack/docker-compose.yml`

```yaml
version: '3.8'

services:
  ai-security-stack:
    build:
      context: ../..
      dockerfile: deployment/security-stack/Dockerfile
    ports:
      - "8000:8000"  # Management Plane
      - "8001:8001"  # Control Plane
    environment:
      - GOOGLE_API_KEY=${GOOGLE_API_KEY}
      - SUPABASE_URL=${SUPABASE_URL}
      - SUPABASE_JWT_SECRET=${SUPABASE_JWT_SECRET}
      - DATA_PLANE_GRPC_URL=${DATA_PLANE_GRPC_URL:-localhost:50051}
      - MANAGEMENT_PLANE_URL=${MANAGEMENT_PLANE_URL:-http://localhost:8000}
    volumes:
      - security-data:/app/data
      - security-models:/root/.cache/huggingface
    restart: unless-stopped

volumes:
  security-data:
    driver: local
  security-models:
    driver: local
```

**Step 3: Test docker-compose**

Run: `cd deployment/security-stack && cp .env.example .env && docker-compose config`
Expected: Valid docker-compose configuration printed

**Step 4: Commit**

```bash
git add deployment/security-stack/docker-compose.yml deployment/security-stack/.env.example
git commit -m "feat(deployment): add AI Security Stack docker-compose configuration"
```

---

### Task 7: UI Console Dockerfile

**Files:**
- Create: `deployment/ui/Dockerfile`
- Create: `deployment/ui/.dockerignore`
- Create: `deployment/ui/nginx.conf`

**Step 1: Create .dockerignore**

File: `deployment/ui/.dockerignore`

```
node_modules
dist
.env
*.log
.git
```

**Step 2: Create nginx.conf**

File: `deployment/ui/nginx.conf`

```nginx
server {
    listen 80;
    server_name _;

    root /usr/share/nginx/html;
    index index.html;

    # Gzip compression
    gzip on;
    gzip_vary on;
    gzip_min_length 1024;
    gzip_types text/plain text/css text/xml text/javascript application/javascript application/json application/xml+rss;

    # Security headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Cache static assets
    location ~* \.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    # SPA fallback routing
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy\n";
        add_header Content-Type text/plain;
    }
}
```

**Step 3: Create Dockerfile**

File: `deployment/ui/Dockerfile`

```dockerfile
# Build stage
FROM node:18-alpine AS builder

WORKDIR /build

# Copy package files
COPY mcp-ui/package*.json ./

# Install dependencies
RUN npm ci

# Copy source code
COPY mcp-ui/ ./

# Build arguments for environment variables
ARG VITE_API_BASE_URL
ARG VITE_GATEWAY_BASE_URL
ARG VITE_SUPABASE_URL
ARG VITE_SUPABASE_ANON_KEY

# Set environment variables for build
ENV VITE_API_BASE_URL=${VITE_API_BASE_URL}
ENV VITE_GATEWAY_BASE_URL=${VITE_GATEWAY_BASE_URL}
ENV VITE_SUPABASE_URL=${VITE_SUPABASE_URL}
ENV VITE_SUPABASE_ANON_KEY=${VITE_SUPABASE_ANON_KEY}

# Build the app
RUN npm run build

# Production stage
FROM nginx:alpine

# Copy custom nginx config
COPY deployment/ui/nginx.conf /etc/nginx/conf.d/default.conf

# Copy built static files
COPY --from=builder /build/dist /usr/share/nginx/html

# Expose HTTP port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --retries=3 \
  CMD wget --quiet --tries=1 --spider http://localhost/health || exit 1

# Run nginx
CMD ["nginx", "-g", "daemon off;"]
```

**Step 4: Test build**

Run: `cd deployment/ui && docker build -t tupl-ui:test -f Dockerfile --build-arg VITE_API_BASE_URL=http://localhost:8000 --build-arg VITE_GATEWAY_BASE_URL=http://localhost:3000 --build-arg VITE_SUPABASE_URL=https://test.supabase.co --build-arg VITE_SUPABASE_ANON_KEY=test-key ../..`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add deployment/ui/Dockerfile deployment/ui/.dockerignore deployment/ui/nginx.conf
git commit -m "feat(deployment): add UI Console Dockerfile and Nginx config"
```

---

### Task 8: UI Console docker-compose.yml

**Files:**
- Create: `deployment/ui/docker-compose.yml`
- Create: `deployment/ui/.env.example`

**Step 1: Create .env.example**

File: `deployment/ui/.env.example`

```bash
# Backend Service URLs
VITE_API_BASE_URL=https://api.tupl.io
VITE_GATEWAY_BASE_URL=https://gateway.tupl.io

# Supabase Configuration
VITE_SUPABASE_URL=https://your-project.supabase.co
VITE_SUPABASE_ANON_KEY=your-anon-key-here
```

**Step 2: Create docker-compose.yml**

File: `deployment/ui/docker-compose.yml`

```yaml
version: '3.8'

services:
  tupl-ui:
    build:
      context: ../..
      dockerfile: deployment/ui/Dockerfile
      args:
        - VITE_API_BASE_URL=${VITE_API_BASE_URL}
        - VITE_GATEWAY_BASE_URL=${VITE_GATEWAY_BASE_URL}
        - VITE_SUPABASE_URL=${VITE_SUPABASE_URL}
        - VITE_SUPABASE_ANON_KEY=${VITE_SUPABASE_ANON_KEY}
    ports:
      - "80:80"
    restart: unless-stopped
```

**Step 3: Test docker-compose**

Run: `cd deployment/ui && cp .env.example .env && docker-compose config`
Expected: Valid docker-compose configuration printed

**Step 4: Commit**

```bash
git add deployment/ui/docker-compose.yml deployment/ui/.env.example
git commit -m "feat(deployment): add UI Console docker-compose configuration"
```

---

### Task 9: Create Deployment README Files

**Files:**
- Create: `deployment/gateway/README.md`
- Create: `deployment/security-stack/README.md`
- Create: `deployment/ui/README.md`

**Step 1: Create Gateway README**

File: `deployment/gateway/README.md`

```markdown
# MCP Gateway Deployment

## Prerequisites

- Docker and docker-compose installed
- Gemini API key

## Setup

1. Copy environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and set:
   - `GEMINI_API_KEY` - Your Gemini API key

3. Build and start:
   ```bash
   docker-compose up -d
   ```

4. Check health:
   ```bash
   curl http://localhost:3000/health
   ```

## Volumes

- `gateway-tenants` - Tenant-isolated configurations and workspaces
- `gateway-workspace` - Shared workspace files
- `gateway-chromadb` - Intelligence layer vector store

## Ports

- `3000` - HTTP API endpoint
- `8001` - ChromaDB (internal)

## Logs

```bash
docker-compose logs -f mcp-gateway-http
docker-compose logs -f chromadb
```

## Stop

```bash
docker-compose down
```

## Backup

```bash
# Backup tenant data
docker run --rm -v gateway-tenants:/data -v $(pwd):/backup alpine tar czf /backup/tenants-backup.tar.gz /data

# Backup ChromaDB
docker run --rm -v gateway-chromadb:/data -v $(pwd):/backup alpine tar czf /backup/chromadb-backup.tar.gz /data
```
```

**Step 2: Create Security Stack README**

File: `deployment/security-stack/README.md`

```markdown
# AI Security Stack Deployment

## Prerequisites

- Docker and docker-compose installed
- Google API key (for Gemini)
- Supabase project with JWT secret

## Setup

1. Copy environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and set:
   - `GOOGLE_API_KEY` - Your Google/Gemini API key
   - `SUPABASE_URL` - Your Supabase project URL
   - `SUPABASE_JWT_SECRET` - Your Supabase JWT secret

3. Build and start:
   ```bash
   docker-compose up -d
   ```

4. Check health:
   ```bash
   curl http://localhost:8000/health  # Management Plane
   curl http://localhost:8001/health  # Control Plane
   ```

## Volumes

- `security-data` - SQLite databases, policies, telemetry
- `security-models` - Cached Hugging Face models

## Ports

- `8000` - Management Plane API
- `8001` - Control Plane API
- `50051` - Data Plane gRPC (internal only)

## Logs

```bash
docker-compose logs -f ai-security-stack
```

## Stop

```bash
docker-compose down
```

## Backup

```bash
# Backup security data
docker run --rm -v security-data:/data -v $(pwd):/backup alpine tar czf /backup/security-data-backup.tar.gz /data
```
```

**Step 3: Create UI README**

File: `deployment/ui/README.md`

```markdown
# UI Console Deployment

## Prerequisites

- Docker and docker-compose installed
- Supabase project configured with Google OAuth
- Backend services deployed (Gateway + Security Stack)

## Setup

1. Copy environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` and set:
   - `VITE_API_BASE_URL` - AI Security Stack Management Plane URL
   - `VITE_GATEWAY_BASE_URL` - MCP Gateway URL
   - `VITE_SUPABASE_URL` - Your Supabase project URL
   - `VITE_SUPABASE_ANON_KEY` - Your Supabase anonymous key

3. Build and start:
   ```bash
   docker-compose up -d --build
   ```

4. Access UI:
   ```
   http://localhost
   ```

5. Check health:
   ```bash
   curl http://localhost/health
   ```

## Ports

- `80` - HTTP (UI served by Nginx)

## Logs

```bash
docker-compose logs -f tupl-ui
```

## Stop

```bash
docker-compose down
```

## Notes

- UI is a static SPA, no volumes needed
- To update environment variables, rebuild the image
```

**Step 4: Commit**

```bash
git add deployment/gateway/README.md deployment/security-stack/README.md deployment/ui/README.md
git commit -m "docs(deployment): add deployment README files for all three components"
```

---

## Phase 2: Backend API Endpoints (Week 1-2)

### Task 10: Add Health Endpoint to MCP Gateway

**Files:**
- Modify: `mcp-gateway/src/http-server.ts`
- Test: Manual curl test

**Step 1: Add health endpoint to http-server.ts**

Add after existing route handlers:

```typescript
// Health check endpoint
server.setRequestHandler(HealthRequestSchema, async () => {
  try {
    // Check ChromaDB connectivity if intelligence is enabled
    if (config.intelligence?.enabled) {
      const chromaUrl = process.env.MCP_GATEWAY_CHROMA_URL || 'http://localhost:8001';
      const response = await fetch(`${chromaUrl}/api/v1/heartbeat`);
      if (!response.ok) {
        return {
          status: 'unhealthy',
          checks: {
            chromadb: 'unreachable'
          }
        };
      }
    }

    return {
      status: 'healthy',
      checks: {
        chromadb: config.intelligence?.enabled ? 'ok' : 'disabled'
      }
    };
  } catch (error) {
    return {
      status: 'unhealthy',
      error: error.message
    };
  }
});
```

**Step 2: Define HealthRequestSchema**

Add near top of file with other schemas:

```typescript
const HealthRequestSchema = z.object({
  method: z.literal('GET'),
  url: z.string().regex(/^\/health$/),
});
```

**Step 3: Test locally**

Run: `npm run dev:http`
Then: `curl http://localhost:3000/health`
Expected: `{"status":"healthy","checks":{"chromadb":"ok"}}`

**Step 4: Commit**

```bash
git add mcp-gateway/src/http-server.ts
git commit -m "feat(gateway): add /health endpoint with ChromaDB connectivity check"
```

---

### Task 11: Add Health Endpoints to Management Plane

**Files:**
- Modify: `management-plane/app/main.py`
- Test: Manual curl test

**Step 1: Add health endpoint**

Add to `management-plane/app/main.py`:

```python
from fastapi import FastAPI, HTTPException
import grpc

app = FastAPI()

@app.get("/health")
async def health_check():
    """Health check endpoint - verifies Data Plane gRPC connectivity"""
    try:
        # Check Data Plane gRPC connection
        grpc_url = os.getenv("DATA_PLANE_GRPC_URL", "localhost:50051")
        channel = grpc.insecure_channel(grpc_url)

        # Try to connect (with 5 second timeout)
        grpc.channel_ready_future(channel).result(timeout=5)
        channel.close()

        return {
            "status": "healthy",
            "checks": {
                "data_plane_grpc": "ok"
            }
        }
    except grpc.FutureTimeoutError:
        return {
            "status": "unhealthy",
            "checks": {
                "data_plane_grpc": "timeout"
            }
        }
    except Exception as e:
        return {
            "status": "unhealthy",
            "error": str(e)
        }
```

**Step 2: Test locally**

Run: `cd management-plane && uvicorn app.main:app --reload`
Then: `curl http://localhost:8000/health`
Expected: `{"status":"healthy","checks":{"data_plane_grpc":"ok"}}`

**Step 3: Commit**

```bash
git add management-plane/app/main.py
git commit -m "feat(mgmt-plane): add /health endpoint with Data Plane gRPC check"
```

---

### Task 12: Add Health Endpoint to Control Plane

**Files:**
- Modify: `policy_control_plane/server.py`
- Test: Manual curl test

**Step 1: Add health endpoint**

Add to `policy_control_plane/server.py`:

```python
import os
import requests
from flask import Flask, jsonify

app = Flask(__name__)

@app.route('/health', methods=['GET'])
def health_check():
    """Health check endpoint - verifies Management Plane connectivity"""
    try:
        mgmt_url = os.getenv("MANAGEMENT_PLANE_URL", "http://localhost:8000")
        response = requests.get(f"{mgmt_url}/health", timeout=5)

        if response.status_code == 200:
            return jsonify({
                "status": "healthy",
                "checks": {
                    "management_plane": "ok"
                }
            })
        else:
            return jsonify({
                "status": "unhealthy",
                "checks": {
                    "management_plane": f"status_{response.status_code}"
                }
            }), 503
    except requests.exceptions.Timeout:
        return jsonify({
            "status": "unhealthy",
            "checks": {
                "management_plane": "timeout"
            }
        }), 503
    except Exception as e:
        return jsonify({
            "status": "unhealthy",
            "error": str(e)
        }), 503

if __name__ == '__main__':
    app.run(host='0.0.0.0', port=8001)
```

**Step 2: Test locally**

Run: `cd policy_control_plane && python server.py`
Then: `curl http://localhost:8001/health`
Expected: `{"status":"healthy","checks":{"management_plane":"ok"}}`

**Step 3: Commit**

```bash
git add policy_control_plane/server.py
git commit -m "feat(control-plane): add /health endpoint with Management Plane check"
```

---

## Phase 3: Authentication & API Management (Week 2)

### Task 13: Add Supabase Client to UI

**Files:**
- Modify: `mcp-ui/package.json`
- Create: `mcp-ui/src/lib/supabase.ts`
- Create: `mcp-ui/src/contexts/AuthContext.tsx`

**Step 1: Install Supabase client**

Run: `cd mcp-ui && npm install @supabase/supabase-js`

**Step 2: Create Supabase client singleton**

File: `mcp-ui/src/lib/supabase.ts`

```typescript
import { createClient } from '@supabase/supabase-js'

const supabaseUrl = import.meta.env.VITE_SUPABASE_URL
const supabaseAnonKey = import.meta.env.VITE_SUPABASE_ANON_KEY

if (!supabaseUrl || !supabaseAnonKey) {
  throw new Error('Missing Supabase environment variables')
}

export const supabase = createClient(supabaseUrl, supabaseAnonKey)
```

**Step 3: Create Auth Context**

File: `mcp-ui/src/contexts/AuthContext.tsx`

```typescript
import React, { createContext, useContext, useEffect, useState } from 'react'
import { Session, User } from '@supabase/supabase-js'
import { supabase } from '../lib/supabase'

interface AuthContextType {
  session: Session | null
  user: User | null
  loading: boolean
  signIn: () => Promise<void>
  signOut: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const [session, setSession] = useState<Session | null>(null)
  const [user, setUser] = useState<User | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    // Get initial session
    supabase.auth.getSession().then(({ data: { session } }) => {
      setSession(session)
      setUser(session?.user ?? null)
      setLoading(false)
    })

    // Listen for auth changes
    const {
      data: { subscription },
    } = supabase.auth.onAuthStateChange((_event, session) => {
      setSession(session)
      setUser(session?.user ?? null)
    })

    return () => subscription.unsubscribe()
  }, [])

  const signIn = async () => {
    const { error } = await supabase.auth.signInWithOAuth({
      provider: 'google',
      options: {
        redirectTo: `${window.location.origin}/auth/callback`,
      },
    })
    if (error) throw error
  }

  const signOut = async () => {
    const { error } = await supabase.auth.signOut()
    if (error) throw error
  }

  return (
    <AuthContext.Provider value={{ session, user, loading, signIn, signOut }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth() {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}
```

**Step 4: Test build**

Run: `cd mcp-ui && npm run build`
Expected: Build succeeds

**Step 5: Commit**

```bash
git add mcp-ui/package.json mcp-ui/package-lock.json mcp-ui/src/lib/supabase.ts mcp-ui/src/contexts/AuthContext.tsx
git commit -m "feat(ui): add Supabase authentication client and context"
```

---

### Task 14: Implement Gateway API Key Management

**Files:**
- Create: `mcp-gateway/src/api/keys.ts`
- Create: `mcp-gateway/src/api/tenants.ts`
- Modify: `mcp-gateway/src/http-server.ts`

**Step 1: Create API key management module**

File: `mcp-gateway/src/api/keys.ts`

```typescript
import crypto from 'crypto';
import fs from 'fs/promises';
import path from 'path';

export interface ApiKey {
  key: string;
  tenantId: string;
  createdAt: string;
  description?: string;
}

export class ApiKeyManager {
  constructor(private tenantsRoot: string) {}

  async generateKey(tenantId: string, description?: string): Promise<ApiKey> {
    // Generate secure random key
    const key = `tupl_${crypto.randomBytes(32).toString('hex')}`;

    const apiKey: ApiKey = {
      key,
      tenantId,
      createdAt: new Date().toISOString(),
      description,
    };

    // Load existing keys
    const keys = await this.loadKeys();
    keys.push(apiKey);

    // Save to tenants.json
    await this.saveKeys(keys);

    return apiKey;
  }

  async listKeys(tenantId: string): Promise<ApiKey[]> {
    const keys = await this.loadKeys();
    return keys.filter(k => k.tenantId === tenantId);
  }

  async revokeKey(key: string): Promise<boolean> {
    const keys = await this.loadKeys();
    const filtered = keys.filter(k => k.key !== key);

    if (filtered.length === keys.length) {
      return false; // Key not found
    }

    await this.saveKeys(filtered);
    return true;
  }

  async validateKey(key: string): Promise<string | null> {
    const keys = await this.loadKeys();
    const apiKey = keys.find(k => k.key === key);
    return apiKey?.tenantId ?? null;
  }

  private async loadKeys(): Promise<ApiKey[]> {
    const tenantsFile = path.join(this.tenantsRoot, 'tenants.json');

    try {
      const data = await fs.readFile(tenantsFile, 'utf-8');
      const parsed = JSON.parse(data);
      return parsed.keys || [];
    } catch (error) {
      if ((error as any).code === 'ENOENT') {
        return [];
      }
      throw error;
    }
  }

  private async saveKeys(keys: ApiKey[]): Promise<void> {
    const tenantsFile = path.join(this.tenantsRoot, 'tenants.json');

    // Ensure directory exists
    await fs.mkdir(this.tenantsRoot, { recursive: true });

    await fs.writeFile(
      tenantsFile,
      JSON.stringify({ keys }, null, 2),
      'utf-8'
    );
  }
}
```

**Step 2: Add API endpoints to http-server.ts**

Add to `mcp-gateway/src/http-server.ts`:

```typescript
import { ApiKeyManager } from './api/keys';

const keyManager = new ApiKeyManager(tenantsRoot);

// Generate API key
server.setRequestHandler(GenerateKeyRequestSchema, async (request) => {
  const { tenantId, description } = request.body;
  const apiKey = await keyManager.generateKey(tenantId, description);
  return { apiKey };
});

// List API keys
server.setRequestHandler(ListKeysRequestSchema, async (request) => {
  const { tenantId } = request.query;
  const keys = await keyManager.listKeys(tenantId);
  // Don't return full keys, just metadata
  return {
    keys: keys.map(k => ({
      keyPrefix: k.key.substring(0, 12) + '...',
      createdAt: k.createdAt,
      description: k.description,
    })),
  };
});

// Revoke API key
server.setRequestHandler(RevokeKeyRequestSchema, async (request) => {
  const { key } = request.body;
  const revoked = await keyManager.revokeKey(key);
  return { revoked };
});

// Schemas
const GenerateKeyRequestSchema = z.object({
  method: z.literal('POST'),
  url: z.string().regex(/^\/api\/keys$/),
  body: z.object({
    tenantId: z.string(),
    description: z.string().optional(),
  }),
});

const ListKeysRequestSchema = z.object({
  method: z.literal('GET'),
  url: z.string().regex(/^\/api\/keys$/),
  query: z.object({
    tenantId: z.string(),
  }),
});

const RevokeKeyRequestSchema = z.object({
  method: z.literal('DELETE'),
  url: z.string().regex(/^\/api\/keys\/[^\/]+$/),
  body: z.object({
    key: z.string(),
  }),
});
```

**Step 3: Test locally**

Run: `npm run dev:http`
Test generate: `curl -X POST http://localhost:3000/api/keys -H "Content-Type: application/json" -d '{"tenantId":"test","description":"Test key"}'`
Expected: Returns generated API key

**Step 4: Commit**

```bash
git add mcp-gateway/src/api/keys.ts mcp-gateway/src/http-server.ts
git commit -m "feat(gateway): implement API key management (generate, list, revoke)"
```

---

## Summary

This implementation plan provides:

1. **Phase 1:** Complete deployment infrastructure (Dockerfiles, docker-compose, configs)
2. **Phase 2:** Health endpoints for monitoring
3. **Phase 3:** Authentication (Supabase) and API key management

**Remaining work not included in this plan:**
- UI pages (MCP server management, policy config, API key UI)
- Management Plane policy/boundary CRUD APIs
- Supabase JWT validation in backends
- Full E2E testing

**Next Steps:**
1. Execute Phase 1 to get deployments working
2. Execute Phase 2 to add health monitoring
3. Execute Phase 3 to add auth foundation
4. Create follow-up plan for UI features and policy management

---

**Estimated Timeline:**
- Phase 1: 2-3 days
- Phase 2: 1 day
- Phase 3: 2-3 days

**Total: ~1 week for deployment infrastructure + basic APIs**
