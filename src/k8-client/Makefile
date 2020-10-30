MAKE_DIR = $(dir $(realpath $(firstword $(MAKEFILE_LIST))))

setup:
	kubectl apply -f $(MAKE_DIR)/tests/test.yaml

run-integration-test:
	cargo test --features=k8

run-integration-test-hyper:
	cargo test  --features=k8,hyper_native

# set up cluster roles 
set-anonymous:
	 create clusterrolebinding cluster-system-anonymous --clusterrole=cluster-admin --user=system:anonymous	 


build:
	cargo build

build-isahc:
	cargo build --features="isahc"	

test-hyper-service:
	cargo test test_client_get_services --features="hyper2","k8"

test-hyper-secrets:
	cargo test test_client_secrets --features="hyper2","k8"

test-secrets:
	cargo test test_client_secrets --features=k8

build-test:
	cargo test --no-run --features="hyper2","k8"