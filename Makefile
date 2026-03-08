.PHONY: clean-runtime build run \
	frontend-install frontend-dev frontend-build \
	tauri-preflight tauri-dev tauri-build-desktop \
	tauri-android-init tauri-android-dev tauri-android-build tauri-ios-dev \
	release-check package-linux-x86_64 package-verify upgrade-verify install-verify \
	test test-unit test-integration test-e2e test-all \
	coverage coverage-report coverage-check

clean-runtime:
	bash scripts/clean-runtime.sh

build:
	cargo build -p chaos-bot-backend

run:
	cargo run -p chaos-bot-backend

frontend-install:
	npm --prefix frontend-react install

frontend-dev:
	npm --prefix frontend-react run dev

frontend-build:
	npm --prefix frontend-react run build

tauri-dev:
	npm --prefix frontend-react exec -- tauri dev --config src-tauri/tauri.conf.json

tauri-preflight:
	npm --prefix frontend-react exec -- tauri info

tauri-build-desktop:
	npm --prefix frontend-react exec -- tauri build --config src-tauri/tauri.conf.json --debug --no-bundle

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

tauri-android-init:
	npm --prefix frontend-react exec -- tauri android init --config src-tauri/tauri.conf.json --ci

tauri-android-dev:
	npm --prefix frontend-react exec -- tauri android dev --config src-tauri/tauri.conf.json

tauri-android-build:
	npm --prefix frontend-react exec -- tauri android build --config src-tauri/tauri.conf.json --debug --apk --ci

tauri-ios-dev:
	npm --prefix frontend-react exec -- tauri ios dev --config src-tauri/tauri.conf.json

test: test-unit test-integration

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

test-integration:
	bash scripts/run-test-suite.sh integration cargo test --workspace \
		--test api_integration \
		--test api_routes

test-e2e:
	bash scripts/run-test-suite.sh e2e bash scripts/run-e2e.sh

test-all: test-unit test-integration test-e2e

coverage:
	cargo llvm-cov --workspace --summary-only

coverage-report:
	cargo llvm-cov --workspace --html

coverage-check:
	cargo llvm-cov --workspace --summary-only --fail-under-lines 85
