.PHONY: clean-runtime build run \
	release-check package-linux-x86_64 package-verify upgrade-verify install-verify \
	test test-unit test-cli test-all \
	coverage coverage-report coverage-check

clean-runtime:
	bash scripts/clean-runtime.sh

build:
	cargo build -p chaos-bot-backend --bin chaos-bot

run:
	cargo run -p chaos-bot-backend -- --help

release-check:
	bash scripts/release/validate-version-sync.sh
	bash scripts/release/generate-release-metadata.sh

package-linux-x86_64:
	bash scripts/release/package-linux-x86_64.sh

package-verify:
	bash scripts/release/verify-packaged-runtime.sh

upgrade-verify:
	bash scripts/release/verify-self-upgrade.sh

install-verify:
	bash scripts/release/verify-github-installer.sh

test: test-unit test-cli

test-unit:
	bash scripts/run-test-suite.sh unit cargo test --workspace --lib \
		--test unit_types \
		--test unit_sessions \
		--test unit_memory \
		--test unit_personality \
		--test unit_bootstrap \
		--test unit_config \
		--test unit_logging \
		--test unit_tools \
		--test unit_llm \
		--test unit_agent \
		--test agent_prompt \
		--test tools_symlink

test-cli:
	bash scripts/run-test-suite.sh cli cargo test --workspace \
		--test cli_integration

test-all: test-unit test-cli

coverage:
	cargo llvm-cov --workspace --summary-only

coverage-report:
	cargo llvm-cov --workspace --html

coverage-check:
	cargo llvm-cov --workspace --summary-only --fail-under-lines 85
