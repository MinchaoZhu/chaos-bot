.PHONY: clean-runtime build run \
	test test-unit test-integration test-e2e test-all \
	coverage coverage-report coverage-check

clean-runtime:
	bash scripts/clean-runtime.sh

build:
	cargo build -p chaos-bot-backend

run:
	cargo run -p chaos-bot-backend

test: test-unit test-integration

test-unit:
	cargo test --workspace \
		--test unit_types \
		--test unit_sessions \
		--test unit_memory \
		--test unit_personality \
		--test unit_bootstrap \
		--test unit_config \
		--test unit_tools \
		--test unit_llm \
		--test unit_agent \
		--test agent_prompt \
		--test tools_symlink

test-integration:
	cargo test --workspace \
		--test api_integration \
		--test api_routes

test-e2e:
	cd e2e && test -d node_modules/@playwright/test || npm install
	cd e2e && npx playwright test

test-all: test-unit test-integration test-e2e

coverage:
	cargo llvm-cov --workspace --summary-only

coverage-report:
	cargo llvm-cov --workspace --html

coverage-check:
	cargo llvm-cov --workspace --summary-only --fail-under-lines 85
