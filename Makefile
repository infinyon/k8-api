publish:
	 cargo-publish-all


build-k8-client:
	make -C src/k8-client build

integration-test-k8-client:
	make -C src/k8-client run-integration-test

build-isahc:
	make -C src/k8-client build-isahc


test-hyper-service:
	make -C src/k8-client test-hyper-service

test-isahc:
	make -C src/k8-client test-isahc-secrets
	
build-test:
	make -C src/k8-client build-test



set-minikube-context:
	cargo run --bin k8-ctx-util

build-config-context:
	cd src/k8-config;cargo build --features=context