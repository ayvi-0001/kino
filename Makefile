.ONESHELL:
.SHELLFLAGS += -euo pipefail
SHELL=/usr/bin/bash

.SILENT:

GCP_PROJECT   ?= $(shell gcloud config get-value project 2>/dev/null)
GCP_REGION    ?= us-west1
GCP_ZONES     ?= us-west1-a us-west1-b us-west1-c
AR_REPO       ?= kino
VM_NAME       ?= kino-bot
MACHINE       ?= e2-micro
IMAGE_TAG     ?= latest
IMAGE         := $(GCP_REGION)-docker.pkg.dev/$(GCP_PROJECT)/$(AR_REPO)/kino:$(IMAGE_TAG)
EXISTING_ZONE ?= $(shell gcloud compute instances list --project "$(GCP_PROJECT)" --filter="name=$(VM_NAME)" | grep $(AR_REPO) | tr -s ' ' | cut -d' ' -f2)

ar-repo:
	if ! gcloud artifacts repositories describe $(AR_REPO) \
		--project $(GCP_PROJECT) --location $(GCP_REGION) >/dev/null 2>&1; then
		echo "creating artifact registry repo $(AR_REPO) in $(GCP_REGION)"
		gcloud artifacts repositories create $(AR_REPO) \
			--project $(GCP_PROJECT) --location $(GCP_REGION) \
			--repository-format docker
	fi

docker-build:
	: "$${DISCORD_TOKEN:?set DISCORD_TOKEN before deploying}"
	DOCKER_BUILDKIT=1 docker build -t $(IMAGE) .

docker-push: docker-build ar-repo
	gcloud auth configure-docker $(GCP_REGION)-docker.pkg.dev --quiet
	docker push $(IMAGE)

deploy: docker-push
	if [[ -n "$(EXISTING_ZONE)" ]]; then
		echo "updating $(VM_NAME) in $(EXISTING_ZONE)"
		gcloud compute instances update-container "$(VM_NAME)" \
			--project "$(GCP_PROJECT)" \
			--zone "$(EXISTING_ZONE)" \
			--container-image=$(IMAGE)
	else
		CREATED=
		for zone in $(GCP_ZONES); do
			echo "creating VM $(VM_NAME) in $$zone"
			if gcloud compute instances create-with-container "$(VM_NAME)" \
				--container-env-file .env \
				--container-image "$(IMAGE)" \
				--container-privileged \
				--container-restart-policy always \
				--create-disk auto-delete=yes,device-name=instance-20260826-000000,image=projects/ubuntu-os-cloud/global/images/ubuntu-minimal-2204-jammy-v20260826,mode=rw,size=10,type=pd-standard \
				--scopes cloud-platform \
				--machine-type "$(MACHINE)" \
				--maintenance-policy MIGRATE \
				--network-interface network-tier=PREMIUM,stack-type=IPV4_ONLY,subnet=default \
				--project "$(GCP_PROJECT)" \
				--provisioning-model STANDARD \
				--reservation-affinity any \
				--shielded-integrity-monitoring \
				--shielded-secure-boot \
				--shielded-vtpm \
				--zone "$$zone"; then
				CREATED=$$zone
				break
			fi
			echo "zone $$zone unavailable, trying next"
		done
		if [[ -z "$$CREATED" ]]; then
			echo "all zones exhausted: $(GCP_ZONES)" >&2
			exit 1
		fi
	fi

install-sqlx-cli:
	cargo install sqlx-cli --no-default-features --features native-tls,postgres,sqlite

sqlx-prepare:
	cargo sqlx database setup \
		--database-url "$$POSTGRES_DATABASE_URL" \
		--source migrations/postgres
	cargo sqlx database setup \
		--database-url "$$SQLITE_DATABASE_URL" \
		--source migrations/sqlite \
		--sqlite-create-db-wal true

	cargo sqlx prepare \
		--no-dotenv \
		--database-url "$$POSTGRES_DATABASE_URL" \
		--workspace \
		-- \
		--features postgres \
		--all-targets
	mkdir -p .sqlx/postgres/
	mv .sqlx/*.json .sqlx/postgres/

	cargo sqlx prepare \
		--no-dotenv \
		--database-url "$$SQLITE_DATABASE_URL" \
		--workspace \
		-- \
		--no-default-features \
		--features sqlite \
		--all-targets
	mkdir -p .sqlx/sqlite/
	mv .sqlx/*.json .sqlx/sqlite/

	mv .sqlx/sqlite/* .sqlx/postgres/* .sqlx/
	rm -d .sqlx/sqlite/ .sqlx/postgres/

run:
	cargo watch -w src/ -w crates/ -x 'run --features tmdb --release'

stop:
	gcloud compute instances stop "$(VM_NAME)" \
		--project "$(GCP_PROJECT)" \
		--zone "$(EXISTING_ZONE)"

start:
	gcloud compute instances start "$(VM_NAME)" \
		--project "$(GCP_PROJECT)" \
		--zone "$(EXISTING_ZONE)"

.PHONY: \
	ar-repo \
	docker-build \
	install-sqlx-cli \
	run \
	sqlx-prepare \
	start \
	stop
