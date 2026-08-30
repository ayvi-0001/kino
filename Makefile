.ONESHELL:
.SHELLFLAGS += -euo pipefail
SHELL=/usr/bin/bash

.SILENT:

GCP_PROJECT ?= $(shell gcloud config get-value project 2>/dev/null)
GCP_REGION  ?= us-west1
GCP_ZONES   ?= us-west1-a us-west1-b us-west1-c
AR_REPO     ?= kino
VM_NAME     ?= kino-bot
MACHINE     ?= e2-micro
IMAGE_TAG   ?= latest
IMAGE       := $(GCP_REGION)-docker.pkg.dev/$(GCP_PROJECT)/$(AR_REPO)/kino:$(IMAGE_TAG)

install-sqlx-cli:
	cargo install sqlx-cli --no-default-features --features native-tls,postgres,sqlite

sqlx-prepare:
	cargo sqlx prepare --no-dotenv -- --all-targets --all-features

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
	EXISTING_ZONE=$$(
		gcloud compute instances list \
			--project $(GCP_PROJECT) \
			--filter="name=$(VM_NAME)" \
			--format="value(zone.basename())"
	)

	if [[ -n "$$EXISTING_ZONE" ]]; then
		echo "updating $(VM_NAME) in $$EXISTING_ZONE"
		gcloud compute instances update-container "$(VM_NAME)" \
			--project "$(GCP_PROJECT)" \
			--zone "$$EXISTING_ZONE" \
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

.PHONY: \
	ar-repo \
	deploy \
	docker-build \
	docker-push \
	install-sqlx-cli \
	sqlx-prepare
