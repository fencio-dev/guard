# Guard Platform - Production Deployment Guide

This guide provides step-by-step instructions for deploying the Guard security platform on Amazon EC2 using Docker Compose.

## Prerequisites

Before beginning deployment, ensure your EC2 instance has the following:

### Software Requirements
- **Docker**: Version 20.10 or later
  - Installation: `sudo apt-get install docker.io` (Ubuntu/Debian)
  - Verification: `docker --version`

- **Docker Compose**: Version 2.0 or later
  - Installation: `sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose && sudo chmod +x /usr/local/bin/docker-compose`
  - Verification: `docker-compose --version`

- **Git**: For cloning the repository
  - Installation: `sudo apt-get install git`

- **curl** and **nc** (netcat): For health checks
  - Installation: `sudo apt-get install curl netcat-openbsd`

### Hardware Requirements
- **CPU**: Minimum 4 cores (8+ recommended)
- **RAM**: Minimum 16GB (32GB recommended)
- **Storage**: Minimum 100GB SSD (200GB+ recommended for model caching)
- **Network**: Stable internet connection for API access

### IAM and Security
- **EC2 Security Group**: Configure to allow:
  - Port 22 (SSH): For remote access
  - Port 8001 (Management Plane): Application API
  - Port 50051 (Data Plane): gRPC communication
  - Port 3001 (MCP Server): Protocol server
  - Port 8000 (Chroma): Vector database (internal, optional exposure)
  - Outbound: 443 (HTTPS) for external APIs

- **EC2 Instance Profile**: May need permissions for:
  - CloudWatch Logs (if using CloudWatch)
  - S3 (if using S3 for model storage)
  - Secrets Manager (if using Secrets Manager for credentials)

## Deployment Steps

### Step 1: Transfer BERT Models

The BERT models are required for policy canonicalization and should be transferred from your local development machine.

#### 1.1 Prepare Models Locally

On your local machine, ensure the BERT models are in the correct directory:
```bash
ls -la management_plane/models/bert/
# Should contain: config.json, pytorch_model.bin, tokenizer.json, vocab.txt, etc.
```

#### 1.2 Create Model Archive

```bash
cd management_plane
zip -r models.zip models/
# Verify archive was created
ls -lh models.zip
```

#### 1.3 Transfer to EC2

```bash
# From your local machine
scp -i /path/to/key.pem models.zip ec2-user@your-ec2-ip:~/

# Connect to EC2
ssh -i /path/to/key.pem ec2-user@your-ec2-ip
```

#### 1.4 Extract Models on EC2

```bash
# On EC2 instance
cd /home/ec2-user

# Clone the repository if not already done
git clone <repository-url> guard
cd guard

# Ensure models directory exists
mkdir -p management_plane/models

# Extract the models archive
unzip ~/models.zip -d ./management_plane/

# Verify extraction
ls -la management_plane/models/bert/

# Clean up archive
rm ~/models.zip

# Set appropriate permissions
chmod -R 755 management_plane/models
```

### Step 2: Configure Environment

#### 2.1 Create .env File

```bash
cd /home/ec2-user/guard

# Copy the template
cp .env.example .env  # if available, or create new

# Edit the .env file with production values
nano .env
```

#### 2.2 Required Environment Variables

Ensure the following variables are configured:

```env
# CRITICAL: Must be obtained from Google Cloud Console
GOOGLE_API_KEY=your_actual_google_api_key_here

# Ports (adjust if needed for your infrastructure)
MGMT_PLANE_PORT=8001
DATA_PLANE_PORT=50051
MCP_SERVER_PORT=3001
CHROMA_PORT=8000

# Logging level for production
LOG_LEVEL=INFO
RUST_LOG=info

# Model configuration
GEMINI_MODEL=gemini-1.5-flash
EMBEDDING_MODEL=all-MiniLM-L6-v2

# Database - SQLite for small deployments, PostgreSQL for production
DATABASE_URL=sqlite:///./guard.db

# Feature flags
CANONICALIZATION_ENABLED=true
```

#### 2.3 Secure API Key Storage

For production deployments, use AWS Secrets Manager:

```bash
# Store the API key in Secrets Manager
aws secretsmanager create-secret \
  --name guard/google-api-key \
  --secret-string "your_actual_api_key"

# Update .env to reference the secret
# GOOGLE_API_KEY=$(aws secretsmanager get-secret-value --secret-id guard/google-api-key --query SecretString --output text)
```

### Step 3: Build and Run Services

#### 3.1 Build Docker Images

```bash
# Navigate to repository root
cd /home/ec2-user/guard

# Build all Docker images (this may take 10-15 minutes)
docker-compose build

# Verify images were built successfully
docker images | grep guard
```

#### 3.2 Start Services

```bash
# Start all services in the background
docker-compose up -d --build

# Watch the startup process (Ctrl+C to exit)
docker-compose logs -f
```

#### 3.3 Monitor Startup Sequence

Services should start in order:
1. **chromadb** (10-15 seconds)
2. **data-plane** (waits for chromadb healthy, 15-20 seconds)
3. **management-plane** (waits for data-plane healthy, 30-45 seconds)
4. **mcp-server** (waits for management-plane healthy, 15-20 seconds)

Total startup time: approximately 1-2 minutes

### Step 4: Verify Services

#### 4.1 Check Service Status

```bash
# View all containers and their status
docker-compose ps

# Expected output:
# NAME                       STATUS              PORTS
# guard-chromadb            Up (healthy)        0.0.0.0:8000->8000/tcp
# guard-data-plane          Up (healthy)        0.0.0.0:50051->50051/tcp
# guard-management-plane    Up (healthy)        0.0.0.0:8001->8001/tcp
# guard-mcp-server          Up (healthy)        0.0.0.0:3001->3001/tcp
```

#### 4.2 Health Check API

```bash
# Check Management Plane health
curl -s http://localhost:8001/health | jq

# Expected response:
# {
#   "status": "healthy",
#   "services": {
#     "database": "connected",
#     "vector_store": "connected",
#     "data_plane": "connected"
#   }
# }
```

#### 4.3 Test Chroma Vector Database

```bash
# Check Chroma heartbeat
curl -s http://localhost:8000/api/v1/heartbeat | jq

# Expected response includes Chroma version
```

#### 4.4 View Service Logs

```bash
# View logs for all services
docker-compose logs

# View logs for specific service
docker-compose logs management-plane

# Follow logs in real-time
docker-compose logs -f data-plane

# View last 100 lines of logs
docker-compose logs --tail 100 mcp-server
```

### Step 5: Database Initialization

#### 5.1 Initialize SQLite Database (Automatic)

The SQLite database will be automatically created on first startup. To verify:

```bash
# Check if database file exists
ls -lh guard.db

# Inspect database schema (requires sqlite3)
sudo apt-get install sqlite3
sqlite3 guard.db ".tables"
```

#### 5.2 Optional: PostgreSQL Setup (Production)

For production deployments with higher concurrency requirements:

```bash
# Install PostgreSQL client
sudo apt-get install postgresql-client

# Update DATABASE_URL in .env
# DATABASE_URL=postgresql://user:password@localhost:5432/guard

# Run database migrations (if applicable)
docker-compose exec management-plane python -m alembic upgrade head
```

## Service Access URLs

After successful deployment, the following services are accessible:

| Service | URL | Protocol | Purpose |
|---------|-----|----------|---------|
| Management Plane | http://your-ec2-ip:8001 | HTTP/REST | Policy analysis API |
| Chroma | http://your-ec2-ip:8000 | HTTP/REST | Vector database (admin) |
| Data Plane | grpc://your-ec2-ip:50051 | gRPC | Internal communication |
| MCP Server | http://your-ec2-ip:3001 | HTTP/SSE | Model Context Protocol |

## Monitoring and Maintenance

### Real-time Monitoring

```bash
# Monitor container resource usage
docker stats

# Monitor Docker daemon logs
docker logs --follow

# System resource monitoring
top
free -h
df -h
```

### Log Aggregation

```bash
# Export all service logs to file
docker-compose logs > guard-deployment-$(date +%Y%m%d-%H%M%S).log

# Parse logs for errors
docker-compose logs | grep -i error

# Monitor specific service with timestamp
docker-compose logs -f --timestamps management-plane
```

### Performance Diagnostics

```bash
# Check disk usage
du -sh /var/lib/docker/volumes/*

# Check network connectivity between services
docker-compose exec management-plane curl http://data-plane:50051

# Verify all networks are configured
docker network ls
docker network inspect guard-network
```

## Troubleshooting

### Service Fails to Start

**Problem**: One or more services remain in "restarting" state

**Diagnosis**:
```bash
docker-compose ps
docker-compose logs <service-name>
```

**Common Causes and Solutions**:

1. **Port Already in Use**
   ```bash
   # Find process using port
   sudo lsof -i :8001
   # Kill process or change port in .env
   ```

2. **Build Failure**
   ```bash
   # Rebuild with verbose output
   docker-compose build --no-cache
   ```

3. **Memory Exhaustion**
   ```bash
   # Check available memory
   free -h
   # Increase Docker memory limit in /etc/docker/daemon.json
   ```

### Slow Startup

**Problem**: Services take longer than expected to start

**Investigation**:
```bash
# Monitor startup logs
docker-compose logs --follow

# Check system resources
docker stats

# Increase timeout values in docker-compose.yml if needed
```

### Network Connectivity Issues

**Problem**: Services cannot communicate with each other

**Diagnosis**:
```bash
# Check network exists
docker network ls | grep guard-network

# Test connectivity from one service to another
docker-compose exec management-plane curl http://chromadb:8000/api/v1/heartbeat
docker-compose exec management-plane python -c "import grpc; grpc.secure_channel('data-plane:50051', grpc.ssl_channel_credentials())"

# Check DNS resolution
docker-compose exec management-plane nslookup chromadb
```

### High Memory Usage

**Problem**: Containers consume excessive memory

**Solutions**:
```bash
# Identify memory-hungry container
docker stats

# Reduce model cache or batch size in .env
# Implement memory limits in docker-compose.yml:
# deploy:
#   resources:
#     limits:
#       memory: 8G

# Restart services with resource constraints
docker-compose restart
```

### API Rate Limiting

**Problem**: Getting 429 errors from Google Gemini API

**Solutions**:
1. Verify GOOGLE_API_KEY is correct
2. Check API quota in Google Cloud Console
3. Implement rate limiting in application
4. Consider switching to gemini-1.5-flash if using pro

## Updating Deployment

### Update Code

```bash
# Pull latest changes
cd /home/ec2-user/guard
git pull origin main

# Rebuild images
docker-compose build --no-cache

# Restart services
docker-compose up -d

# Verify new version
docker-compose logs --tail 50
```

### Update Models

```bash
# Stop services
docker-compose down

# Replace models
rm -rf management_plane/models/bert
# Transfer new models (follow Step 1 process)

# Restart services
docker-compose up -d --build
```

### Update Environment Variables

```bash
# Edit .env file
nano .env

# Restart only affected services
docker-compose up -d management-plane

# Verify changes
docker-compose logs -f management-plane
```

## Production Checklist

- [ ] Security group configured correctly
- [ ] GOOGLE_API_KEY stored securely
- [ ] Models transferred and verified
- [ ] All services starting successfully
- [ ] Health checks passing
- [ ] Database initialized
- [ ] API responding correctly
- [ ] Logs being collected properly
- [ ] Monitoring configured
- [ ] Backup strategy implemented
- [ ] SSL/TLS certificates installed (if needed)
- [ ] Rate limiting configured
- [ ] Database backups scheduled

## Rollback Procedure

If deployment encounters issues:

```bash
# Stop all services
docker-compose down

# Remove problematic images
docker rmi guard-management-plane guard-data-plane guard-mcp-server

# Checkout previous working version
git checkout main~1

# Rebuild and restart
docker-compose up -d --build

# Verify
docker-compose ps
```

## Support and Debugging

### Collect Diagnostic Information

```bash
# Create diagnostic bundle
mkdir guard-diagnostics
docker-compose ps > guard-diagnostics/containers.txt
docker-compose logs > guard-diagnostics/logs.txt
docker stats --no-stream > guard-diagnostics/stats.txt
docker images | grep guard > guard-diagnostics/images.txt
env | grep -E "GOOGLE_API_KEY|DATABASE_URL" > guard-diagnostics/env.txt

# Archive for sharing
tar -czf guard-diagnostics.tar.gz guard-diagnostics/
```

### Common Issues Quick Reference

| Issue | Command | Expected Result |
|-------|---------|-----------------|
| Service not running | `docker-compose ps` | Status = "Up" |
| Health check failing | `docker inspect <container> \| grep Health` | State.Health.Status = "healthy" |
| Cannot connect to API | `curl -v http://localhost:8001/health` | HTTP 200 |
| Out of disk space | `df -h` | Used < 80% |
| Memory issues | `docker stats` | Memory usage reasonable |

## Next Steps

1. Configure monitoring (CloudWatch, Prometheus, etc.)
2. Set up log aggregation (ELK Stack, Datadog, etc.)
3. Implement backup strategy
4. Configure SSL/TLS certificates
5. Set up auto-scaling if needed
6. Configure disaster recovery procedures

---

For additional support, refer to the main README.md or contact the development team.
