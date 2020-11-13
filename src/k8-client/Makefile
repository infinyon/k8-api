MAKE_DIR = $(dir $(realpath $(firstword $(MAKEFILE_LIST))))
TARGET_LINUX=x86_64-unknown-linux-musl
DOCKER_TAG=latest

setup:
	kubectl apply -f $(MAKE_DIR)/tests/test.yaml

run-integration-test-native:
	cargo test --features k8,native_tls

run-integration-test-rustls:
	cargo test --no-default-features --features k8,rust_tls

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


build-hello-minikube:
	cargo build --bin hello --target $(TARGET_LINUX)
	echo "Building hello musl image with version: $(DOCKER_TAG)"
	export CARGO_PROFILE=$(if $(RELEASE),release,debug); \
	export MINIKUBE_DOCKER_ENV=true; \
	export DOCKER_TAG=$(DOCKER_TAG); \
	k8-test/build.sh
	kubectl apply -f k8-test/hello-deploy.yaml