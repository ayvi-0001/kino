.ONESHELL:
.SHELLFLAGS += -euo pipefail
SHELL=/usr/bin/bash

.SILENT:

install-sqlx-cli:
	cargo install sqlx-cli --no-default-features --features native-tls,postgres,sqlite

sqlx-prepare:
	cargo sqlx prepare --no-dotenv -- --all-targets --all-features

.PHONY: \
	sqlx-prepare
