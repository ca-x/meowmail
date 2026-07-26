SHELL := /bin/bash
.SHELLFLAGS := -eu -o pipefail -c

.PHONY: dev web test build docker

dev:
	cargo run & backend_pid=$$!; \
	trap 'kill "$$backend_pid" 2>/dev/null || true; wait "$$backend_pid" 2>/dev/null || true' EXIT INT TERM; \
	npm --prefix web run dev

web:
	npm --prefix web ci --ignore-scripts
	npm --prefix web run build

test:
	cargo fmt --all -- --check
	cargo clippy --locked --all-targets --all-features -- -D warnings
	cargo test --locked
	npm --prefix web run typecheck
	npm --prefix web run test:ci

build: web
	cargo build --release --locked

docker:
	docker build --tag meowmail:local .
