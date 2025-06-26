#[cfg(feature = "k8")]
mod integration_tests {

    use fluvio_future::test_async;

    use anyhow::Result;
    use k8_client::K8Client;
    use k8_metadata_client::MetadataClient;
    use k8_types::{
        core::{
            affinity::{
                Affinity, LabelSelector, NodeAffinity, NodeSelector, NodeSelectorTerm, PodAffinity,
                PodAffinityTerm, PreferredSchedulingTerm, SelectorOperator, SelectorRequirement,
                WeightedPodAffinityTerm,
            },
            pod::{ContainerSpec, PodRestartPolicy, PodSpec},
        },
        InputK8Obj, InputObjectMeta,
    };
    use std::collections::HashMap;

    const NS: &str = "default";
    const POD_NAME: &str = "pod-test-name";

    fn create_client() -> K8Client {
        K8Client::try_default().expect("cluster not initialized")
    }

    async fn cleanup_pod(client: &K8Client) {
        let meta = InputObjectMeta {
            name: POD_NAME.to_string(),
            namespace: NS.to_string(),
            ..Default::default()
        };
        let _ = client.delete_item::<PodSpec, _>(&meta).await;
    }

    async fn create_pod(client: &K8Client) {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "test".to_string());

        let input_meta = InputObjectMeta {
            name: POD_NAME.to_string(),
            namespace: NS.to_string(),
            labels,
            ..Default::default()
        };

        let pod = PodSpec {
            containers: vec![ContainerSpec {
                name: POD_NAME.to_string(),
                image: Some("busybox".to_string()),
                command: vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    "echo \"Hello, Kubernetes!\"".to_string(),
                ],
                ..Default::default()
            }],
            restart_policy: Some(PodRestartPolicy::Never),
            affinity: Some(Affinity {
                node_affinity: Some(NodeAffinity {
                    required: Some(NodeSelector {
                        node_selector_terms: vec![NodeSelectorTerm {
                            match_expressions: Some(vec![SelectorRequirement {
                                key: "kubernetes.io/os".to_string(),
                                operator: SelectorOperator::In,
                                values: Some(vec!["linux".to_string()]),
                            }]),
                            match_fields: None,
                        }],
                    }),
                    preferred: Some(vec![PreferredSchedulingTerm {
                        weight: 20,
                        preference: NodeSelectorTerm {
                            match_expressions: Some(vec![SelectorRequirement {
                                key: "node-role.kubernetes.io/control-plane".to_string(),
                                operator: SelectorOperator::DoesNotExist,
                                values: None,
                            }]),
                            match_fields: None,
                        },
                    }]),
                }),
                pod_affinity: Some(PodAffinity {
                    required: None, // Remove required pod affinity for single-node compatibility
                    preferred: Some(vec![WeightedPodAffinityTerm {
                        weight: 50,
                        pod_affinity_term: PodAffinityTerm {
                            label_selector: Some(LabelSelector {
                                match_expressions: Some(vec![SelectorRequirement {
                                    key: "component".to_string(),
                                    operator: SelectorOperator::In,
                                    values: Some(vec!["web".to_string()]),
                                }]),
                                match_labels: None,
                            }),
                            namespace_selector: None,
                            namespaces: Some(vec!["default".to_owned()]),
                            topology_key: "kubernetes.io/zone".to_string(),
                        },
                    }]),
                }),
                pod_anti_affinity: Some(PodAffinity {
                    required: Some(vec![PodAffinityTerm {
                        label_selector: Some(LabelSelector {
                            match_expressions: None,
                            match_labels: Some(
                                [("app".to_owned(), "test".to_owned())]
                                    .iter()
                                    .cloned()
                                    .collect(),
                            ),
                        }),
                        namespace_selector: None,
                        namespaces: Some(vec!["default".to_owned()]),
                        topology_key: "kubernetes.io/hostname".to_string(),
                    }]),
                    preferred: None,
                }),
            }),
            ..Default::default()
        };

        let input = InputK8Obj::new(pod, input_meta);

        client.apply(input).await.expect("failed creating pod");
    }

    async fn check_pod(client: &K8Client) {
        let pod_items = client
            .retrieve_items::<PodSpec, _>(NS)
            .await
            .expect("pods should exist");

        assert_eq!(pod_items.items.len(), 1);
        for pod in pod_items.items {
            assert_eq!(pod.metadata.name, POD_NAME);
        }
    }
    #[test_async]
    async fn test_pod_affinity() -> Result<()> {
        let client = create_client();

        // Clean up any existing pod first
        cleanup_pod(&client).await;

        create_pod(&client).await;
        check_pod(&client).await;

        // Clean up after test
        cleanup_pod(&client).await;

        Ok(())
    }
}
