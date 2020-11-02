MAKE_DIR = $(dir $(realpath $(firstword $(MAKEFILE_LIST))))

setup:
	kubectl apply -f $(MAKE_DIR)/tests/test.yaml

run-integration-test:
	cargo test --features=k8

run-test-service-changes:
	- kubectl get services --no-headers=true | awk '/testservice/{print $1}' | xargs  kubectl delete service
	sleep 1
	cargo test --features=k8 test_service_changes

# set up cluster roles 
set-anonymous:
	 create clusterrolebinding cluster-system-anonymous --clusterrole=cluster-admin --user=system:anonymous	 


build:
	cargo build



test-hyper-service:
	cargo test test_client_get_services --features="hyper2","k8"

test-hyper-secrets:
	cargo test test_client_secrets --features="hyper2","k8"

test-secrets:
	cargo test test_client_secrets --features=k8

build-test:
	cargo test --no-run --features="hyper2","k8"