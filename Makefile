SHELL := /bin/bash

LOOP_MIN_AVAILABLE_MIB ?= 3072

.PHONY: check lint typecheck test coverage rustdoc-check py-integration-gate ignored-repl-gate proptest-gate claude-adapter-gate review verify

check: typecheck test

lint:
	cd rlm-core && cargo fmt --check

typecheck:
	cd rlm-core && cargo check --quiet

test:
	cd rlm-core && cargo test --quiet

coverage:
	./scripts/run_coverage.sh

rustdoc-check:
	cd rlm-core && RUSTDOCFLAGS="-D warnings" cargo doc --no-deps

py-integration-gate:
	LOOP_MIN_AVAILABLE_MIB=$(LOOP_MIN_AVAILABLE_MIB) ./scripts/run_vg_py_integration_gate.sh

ignored-repl-gate:
	LOOP_MIN_AVAILABLE_MIB=$(LOOP_MIN_AVAILABLE_MIB) ./scripts/safe_run.sh bash -lc 'cd /Users/rand/src/loop/rlm-core && cargo test --no-default-features --features gemini test_repl_spawn -- --ignored --test-threads=1 && cargo test --no-default-features --features gemini test_lean_repl_spawn -- --ignored --test-threads=1'

proptest-gate:
	LOOP_MIN_AVAILABLE_MIB=$(LOOP_MIN_AVAILABLE_MIB) ./scripts/run_vg_proptest_gate.sh

claude-adapter-gate:
	LOOP_MIN_AVAILABLE_MIB=$(LOOP_MIN_AVAILABLE_MIB) ./scripts/run_vg_claude_adapter_e2e_gate.sh

review: typecheck rustdoc-check

verify: check rustdoc-check py-integration-gate proptest-gate claude-adapter-gate
