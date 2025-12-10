.PHONY: all setup build up down logs status clean help deploy
.PHONY: build-gateway build-security build-ui
.PHONY: up-gateway up-security up-ui
.PHONY: down-gateway down-security down-ui
.PHONY: logs-gateway logs-security logs-ui

# Default target
all: help

help:
	@echo "Tupl Deployment Management"
	@echo ""
	@echo "Usage:"
	@echo "  make setup    - Initialize .env files from examples (does not overwrite existing)"
	@echo "  make build    - Build all service images"
	@echo "  make up       - Start all services in detached mode"
	@echo "  make deploy   - Build and start all services (build + up)"
	@echo "  make down     - Stop all services"
	@echo "  make status   - Show status of all containers"
	@echo "  make clean    - Stop services and remove volumes (WARNING: Data loss)"
	@echo ""
	@echo "Service specific commands:"
	@echo "  make [build|up|down|logs]-gateway"
	@echo "  make [build|up|down|logs]-security"
	@echo "  make [build|up|down|logs]-ui"

setup:
	@echo "Checking environment files..."
	@if [ ! -f gateway/.env ]; then \
		cp gateway/.env.example gateway/.env; \
		echo "Created gateway/.env"; \
	else \
		echo "gateway/.env already exists"; \
	fi
	@if [ ! -f security-stack/.env ]; then \
		cp security-stack/.env.example security-stack/.env; \
		echo "Created security-stack/.env"; \
	else \
		echo "security-stack/.env already exists"; \
	fi
	@if [ ! -f ui/.env ]; then \
		cp ui/.env.example ui/.env; \
		echo "Created ui/.env"; \
	else \
		echo "ui/.env already exists"; \
	fi
	@echo "Setup complete. Please ensure .env files are configured correctly."

# Build Targets
build: build-gateway build-security build-ui

build-gateway:
	@echo "Building Gateway..."
	cd gateway && docker-compose build

build-security:
	@echo "Building Security Stack..."
	cd security-stack && docker-compose build

build-ui:
	@echo "Building UI..."
	cd ui && docker-compose build

# Up Targets
up: up-gateway up-security up-ui

up-gateway:
	@echo "Starting Gateway..."
	cd gateway && docker-compose up -d

up-security:
	@echo "Starting Security Stack..."
	cd security-stack && docker-compose up -d

up-ui:
	@echo "Starting UI..."
	cd ui && docker-compose up -d

# Combined Deploy Target
deploy: build up

# Down Targets
down: down-ui down-security down-gateway

down-gateway:
	@echo "Stopping Gateway..."
	-cd gateway && docker-compose down

down-security:
	@echo "Stopping Security Stack..."
	-cd security-stack && docker-compose down

down-ui:
	@echo "Stopping UI..."
	-cd ui && docker-compose down

# Logs Targets
logs:
	@echo "To view logs, run specific targets:"
	@echo "  make logs-gateway"
	@echo "  make logs-security"
	@echo "  make logs-ui"

logs-gateway:
	cd gateway && docker-compose logs -f

logs-security:
	cd security-stack && docker-compose logs -f

logs-ui:
	cd ui && docker-compose logs -f

# Status Target
status:
	@echo "--- Gateway ---"
	@cd gateway && docker-compose ps
	@echo ""
	@echo "--- Security Stack ---"
	@cd security-stack && docker-compose ps
	@echo ""
	@echo "--- UI ---"
	@cd ui && docker-compose ps

# Clean Target
clean:
	@echo "WARNING: This will stop all services and remove all persistent volumes."
	@read -p "Are you sure? [y/N] " ans && [ $${ans:-N} = y ]
	-cd ui && docker-compose down -v
	-cd security-stack && docker-compose down -v
	-cd gateway && docker-compose down -v
	@echo "Clean complete."
