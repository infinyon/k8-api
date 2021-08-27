use serde::Deserialize;
use serde::Serialize;

use crate::core::pod::PodSpec;
use crate::{Crd, CrdNames, DefaultHeader, LabelSelector, Spec, Status, TemplateSpec};

const DEPLOYMENT_API: Crd = Crd {
    group: "apps",
    version: "v1",
    names: CrdNames {
        kind: "Deployment",
        plural: "deployments",
        singular: "deployment",
    },
};

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DeploymentSpec {
    pub min_ready_seconds: Option<i32>,
    pub paused: Option<bool>,
    pub progress_deadline_seconds: Option<i32>,
    pub replicas: Option<i32>,
    pub revision_history_limit: Option<i32>,
    pub selector: LabelSelector,
    pub strategy: Option<DeploymentStrategy>,
    pub template: TemplateSpec<PodSpec>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DeploymentStrategy {
    pub rolling_update: Option<RollingUpdateDeployment>,
    pub type_: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RollingUpdateDeployment {
    pub max_surge: Option<String>,
    pub max_unavailable: Option<String>,
}

impl Spec for DeploymentSpec {
    type Status = DeploymentStatus;
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &DEPLOYMENT_API
    }

    fn make_same(&mut self, _other: &Self) {
        todo!();
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStatus {
    pub available_replicas: Option<i32>,
    pub collision_count: Option<i32>,
    pub conditions: Option<Vec<DeploymentCondition>>,
    pub observed_generation: Option<i64>,
    pub ready_replicas: Option<i32>,
    pub replicas: Option<i32>,
    pub unavailable_replicas: Option<i32>,
    pub updated_replicas: Option<i32>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentCondition {
    pub last_transition_time: Option<String>,
    pub last_update_time: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub status: String,
    pub type_: String,
}

impl Status for DeploymentStatus {}

/*
#[cfg(test)]
mod test {

    use serde_json;
    use serde_json::json;

    use super::LabelSelector;
    use super::StatefulSetSpec;
    use k8_diff::Changes;
    use k8_metadata::cluster::ClusterSpec;
    use k8_metadata::cluster::Configuration;
    use k8_metadata::cluster::Cluster;
    use k8_metadata::cluster::ClusterEndpoint;

    #[test]
    fn test_label_selector() {
        let selector = LabelSelector::new_labels(vec![("app".to_owned(), "test".to_owned())]);

        let maps = selector.match_labels;
        assert_eq!(maps.len(), 1);
        assert_eq!(maps.get("app").unwrap(), "test");
    }

    #[test]
    fn test_cluster_to_stateful() {
        let cluster = ClusterSpec {
            cluster: Cluster {
                replicas: Some(3),
                rack: Some("rack1".to_string()),
                public_endpoint: Some(ClusterEndpoint::new(9005)),
                private_endpoint: Some(ClusterEndpoint::new(9006)),
                controller_endpoint: Some(ClusterEndpoint::new(9004)),
            },
            configuration: Some(Configuration::default()),
            env: None,
        };

        let stateful: StatefulSetSpec = (&cluster).into();
        assert_eq!(stateful.replicas, Some(3));
        let mut stateful2 = stateful.clone();
        stateful2.replicas = Some(2);

        let state1_json = serde_json::to_value(stateful).expect("json");
        let state2_json = serde_json::to_value(stateful2).expect("json");
        let diff = state1_json.diff(&state2_json).expect("diff");
        let json_diff = serde_json::to_value(diff).unwrap();
        assert_eq!(
            json_diff,
            json!({
                "replicas": 2
            })
        );
    }


    /*
    * TODO: make this as utility
    use std::io::Read;
    use std::fs::File;
    use k8_metadata_core::metadata::ObjectMeta;
    use k8_metadata_core::metadata::K8Obj;
    use super::StatefulSetStatus;
    use super::TemplateSpec;
    use super::PodSpec;
    use super::ContainerSpec;
    use super::ContainerPortSpec;

    #[test]
    fn test_decode_statefulset()  {
        let file_name = "/private/tmp/f1.json";

        let mut f = File::open(file_name).expect("open failed");
        let mut contents = String::new();
        f.read_to_string(&mut contents).expect("read file");
       // let st: StatefulSetSpec = serde_json::from_slice(&buffer).expect("error");
        let st: K8Obj<StatefulSetSpec,StatefulSetStatus> = serde_json::from_str(&contents).expect("error");
        println!("st: {:#?}",st);
        assert!(true);
    }
    */

}
*/
