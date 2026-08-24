.ONESHELL:
.SHELLFLAGS += -euo pipefail
SHELL=/usr/bin/bash

.SILENT:

sqlx-prepare:
	cargo sqlx prepare -- --all-targets --all-features --workspace

.PHONY: \
	sqlx-prepare
