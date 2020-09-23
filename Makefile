publish:
	 cargo-publish-all


build-hyper:
	make -C src/k8-client build-hyper


build-isahc:
	make -C src/k8-client build-isahc


test-hyper-service:
	make -C src/k8-client test-hyper-service

test-isahc:
	make -C src/k8-client test-isahc-secrets
	
build-test:
	make -C src/k8-client build-test

integration-test:
	make -C src/k8-client run-integration-test

set-minikube-context:
	cargo run --bin k8-ctx-util

build-config-context:
	cd src/k8-config;cargo build --features=context