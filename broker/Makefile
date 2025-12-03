.PHONY: help
help:
	@echo "No helps today, check the Makefile 😛"

.PHONY: dev
dev:
	air -c ./.dev/.air.toml

.PHONY: staging
staging:
	docker compose \
		--file ./deploy/compose.staging.yaml \
		--env-file ./deploy/staging.env \
		up --detach

.PHONY: staging.fresh
staging.fresh:
	\
		docker compose \
		--file ./deploy/compose.staging.yaml \
		--env-file ./deploy/staging.env \
		pull backend && \
		\
		docker compose \
		--file ./deploy/compose.staging.yaml \
		--env-file ./deploy/staging.env \
		up backend --detach --force-recreate --no-deps
