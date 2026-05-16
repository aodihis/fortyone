.PHONY: dev be fe build build-be build-fe test check

# Run backend and frontend in parallel
dev:
	$(MAKE) -j2 be fe

# Backend: cargo run (reads be/.env automatically via dotenvy)
be:
	cd be && cargo run

# Frontend: trunk serve (reads fe/.env via build.rs)
fe:
	cd fe && trunk serve

# Release builds
build: build-be build-fe

build-be:
	cd be && cargo build --release

build-fe:
	cd fe && trunk build --release

# Tests (backend only — frontend has no test suite)
test:
	cd be && cargo test

# Quick compile check without producing artifacts
check:
	cd be && cargo check
	cd fe && trunk build 2>&1 | grep -E "error|warning|success|Finished"
