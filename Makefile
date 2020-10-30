publish:
	 cargo-publish-all


build-k8-client:
	make -C src/k8-client build

integration-test-k8-client:
	make -C src/k8-client run-integration-test

	
build-test:
	make -C src/k8-client build-test
