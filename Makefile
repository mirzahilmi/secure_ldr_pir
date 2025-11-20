.PHONY: broker
broker:
	cd ./broker/ && \
		docker build --tag ghcr.io/mirzahilmi/secure_ldr_pir_broker:latest .

.PHONY: broker.dev
broker.dev:
	cd ./broker/ && make dev

.PHONY: broker.staging
broker.staging:
	cd ./broker/ && make staging

.PHONY: broker.fresh
broker.fresh:
	cd ./broker/ && make staging.fresh
