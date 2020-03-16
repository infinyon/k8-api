 run-integration-test:
	cargo test --features=k8

# set up cluster roles 
set-anonymous:
	 create clusterrolebinding cluster-system-anonymous --clusterrole=cluster-admin --user=system:anonymous	 


build-hyper:
	cargo build --features="hyper2"

build-isahc:
	cargo build --features="isahc"	

test-hyper-service:
	cargo test test_client_get_services --features="hyper2","k8"

test-hyper-secrets:
	cargo test test_client_secrets --features="hyper2","k8"

test-isahc-secrets:
	cargo test test_client_secrets --features="native","k8"

build-test:
	cargo test --no-run --features="hyper2","k8"