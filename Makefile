publish:
	 cargo-publish-all


k8-client-build:
	make -C src/k8-client build

k8-client-integration-test-native:
	make -C src/k8-client run-integration-test-native


k8-client-integration-test-rustls:
	cargo run --bin k8-ctx-util
	make -C src/k8-client run-integration-test-rustls

k8-client-run-test-service-changes:
	make -C src/k8-client run-test-service-changes
	