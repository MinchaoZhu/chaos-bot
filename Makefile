.PHONY: clean-runtime build run

clean-runtime:
	bash scripts/clean-runtime.sh

build:
	cargo build -p chaos-bot-backend

run:
	cargo run -p chaos-bot-backend
