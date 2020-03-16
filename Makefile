build-hyper:
	cd src/k8-client; make build-hyper


build-isahc:
	cd src/k8-client; make build-isahc


test-hyper-service:
	cd src/k8-client; make test-hyper-service

test-isahc:
	cd src/k8-client; make test-isahc-secrets
	
build-test:
	cd src/k8-client; make build-test