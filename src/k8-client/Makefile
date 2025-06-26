run-integration-test-native:
	cargo test test_pods --features k8,native_tls
	cargo test test_object_conflict --features k8,native_tls
	cargo test test_object_replace --features k8,native_tls
	cargo test test_job_created --features k8,native_tls
	cargo test test_service_changes --features k8,native_tls
	cargo test test_pods_affinity --features k8,native_tls
