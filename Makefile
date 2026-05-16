.PHONY: dev be fe build build-be build-fe test check \
        docker-build-be docker-build-fe docker-push docker-up docker-up-prod

REGISTRY ?= ghcr.io/aodihis
BE_TAG   ?= latest
FE_TAG   ?= latest

# Load fe/.env so SERVE_PORT and SERVE_ADDRESS are available as Make variables.
# The -include means this won't error if the file doesn't exist yet.
-include fe/.env
SERVE_PORT    ?= 8000
SERVE_ADDRESS ?= 127.0.0.1

# Run backend and frontend in parallel
dev:
	$(MAKE) -j2 be fe

# Backend: cargo run (reads be/.env automatically via dotenvy)
be:
	cd be && cargo run

# Frontend: trunk serve — port and address come from fe/.env
fe:
	cd fe && trunk serve --port $(SERVE_PORT) --address $(SERVE_ADDRESS)

# Release builds
build: build-be build-fe

build-be:
	cd be && cargo build --release

build-fe:
	cd fe && trunk build --release

# Tests (backend only)
test:
	cd be && cargo test

# Quick compile check
check:
	cd be && cargo check
	cd fe && cargo check --target wasm32-unknown-unknown

# Docker — build images locally
docker-build-be:
	docker build -t $(REGISTRY)/fortyone-be:$(BE_TAG) ./be

docker-build-fe:
	docker build --build-arg API_URL=$(API_URL) -t $(REGISTRY)/fortyone-fe:$(FE_TAG) ./fe

# Build both and push to GHCR
# Usage: make docker-push API_URL=https://api.yourdomain.com
docker-push: docker-build-be docker-build-fe
	docker push $(REGISTRY)/fortyone-be:$(BE_TAG)
	docker push $(REGISTRY)/fortyone-fe:$(FE_TAG)

# Run locally with Docker Compose
docker-up:
	docker compose up -d

# Run on VPS (pull from GHCR, external Traefik)
docker-up-prod:
	docker compose -f docker-compose.prod.yml up -d
