.PHONY: dev be fe build build-be build-fe test check

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
