SHELL := /bin/bash

.PHONY: check lint typecheck test review verify

check: typecheck test

lint:
	cd rlm-core && cargo fmt --check

typecheck:
	cd rlm-core && cargo check --quiet

test:
	cd rlm-core && cargo test --quiet

review: typecheck

verify: check
