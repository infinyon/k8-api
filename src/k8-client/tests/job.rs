#[cfg(feature = "k8")]
mod integration_tests {

    use fluvio_future::test_async;

    use k8_client::{ClientError, K8Client};
    use k8_metadata_client::MetadataClient;
    use k8_types::{
        InputK8Obj, InputObjectMeta, TemplateSpec,
        batch::job::JobSpec,
        core::pod::{ContainerSpec, PodSpec},
    };

    const NS: &str = "default";
    const JOB_NAME: &str = "job-test-name";

    fn create_client() -> K8Client {
        K8Client::default().expect("cluster not initialized")
    }

    async fn create_job(client: &K8Client) {
        let input_meta = InputObjectMeta {
            name: JOB_NAME.to_string(),
            namespace: NS.to_string(),
            ..Default::default()
        };

        let job = JobSpec {
            template: TemplateSpec::new(PodSpec {
                containers: vec![ContainerSpec {
                    name: JOB_NAME.to_string(),
                    image: Some("busybox".to_string()),
                    command: vec![
                        "sh".to_string(),
                        "-c".to_string(),
                        "echo \"Hello, Kubernetes!\"".to_string(),
                    ],
                    ..Default::default()
                }],
                restart_policy: Some("Never".to_string()),
                ..Default::default()
            }),
            active_deadline_seconds: Some(60),
            backoff_limit: Some(1),
            ..Default::default()
        };

        let input = InputK8Obj::new(job, input_meta);

        client.apply(input).await.expect("failed creating job");
    }

    async fn check_job(client: &K8Client) {
        let job_items = client
            .retrieve_items::<JobSpec, _>(NS)
            .await
            .expect("jobs should exist");

        assert_eq!(job_items.items.len(), 1);
        for job in job_items.items {
            assert_eq!(job.metadata.name, JOB_NAME);
        }
    }
    #[test_async]
    async fn test_job_created() -> Result<(), ClientError> {
        let client = create_client();

        create_job(&client).await;
        check_job(&client).await;

        Ok(())
    }
}
