PWD=$(shell pwd)
TEST=

coverage:
	cargo tarpaulin --ignore-tests

lint:
	cargo clippy -- -D warnings

fmt-check:
	cargo fmt -- --check

audit:
	cargo audit

fmt:
	cargo fmt

cargo-ci:
	cargo watch -x fmt -x check -x test -x run

docker-up:
	docker-compose --env-file="${PWD}/.env" up -d
	sleep 3

up:	docker-up cargo-ci

test_one:
	cargo test -- --show-output "${TEST}" | bunyan

down:
	docker-compose down -v --remove-orphans

clean: coverage lint fmt fmt-check audit