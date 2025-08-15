# Run Kora in Docker (no metrics)
run-docker:
	docker compose down
	docker compose build --no-cache kora
	docker compose up

# Update metrics configuration from kora.toml
update-metrics-config:
	@echo "Updating metrics configuration from kora.toml..."
	@cargo run -p kora-lib --bin update-config

# Run metrics (Prometheus + Grafana) - automatically updates config first
run-metrics: update-metrics-config
	cd crates/lib/src/metrics && docker compose -f docker-compose.metrics.yml down
	cd crates/lib/src/metrics && docker compose -f docker-compose.metrics.yml up

# install ts sdk