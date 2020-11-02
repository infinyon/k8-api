publish:
	 cargo-publish-all


k8-client-build:
	make -C src/k8-client build

k8-client-integration-test:
	make -C src/k8-client run-integration-test

k8-client-run-test-service-changes:
	make -C src/k8-client run-test-service-changes
	
build-test:
	make -C src/k8-client build-test
