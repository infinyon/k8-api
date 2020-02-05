use serde::Deserialize;
use serde::Serialize;

use k8_obj_metadata::Env;
use k8_obj_metadata::Crd;
use k8_obj_metadata::CrdNames;
use k8_obj_metadata::Spec;
use k8_obj_metadata::Status;


//
// Pod Object

const POD_API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "Pod",
        plural: "pods",
        singular: "pod",
    },
};

impl Spec for PodSpec {

    type Status = PodStatus;

    fn metadata() -> &'static Crd {
        &POD_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodSpec {
    pub volumes: Option<Vec<VolumeSpec>>,
    pub containers: Vec<ContainerSpec>,
    pub restart_policy: Option<String>, // TODO; should be enum
    pub service_account_name: Option<String>,
    pub service_account: Option<String>,
    pub node_name: Option<String>,
    pub termination_grace_period_seconds: Option<u16>,
    pub dns_policy: Option<String>,
    pub security_context: Option<PodSecurityContext>,
    pub scheduler_name: Option<String>
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodSecurityContext {
    pub fs_group: Option<u32>,
    pub run_as_group: Option<u32>,
    pub run_as_non_root: Option<bool>,
    pub run_as_user: Option<u32>
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerSpec {
    pub name: String,
    pub ports: Vec<ContainerPortSpec>,
    pub image: Option<String>,
    pub image_pull_policy: Option<String>, // TODO: should be enum
    pub volume_mounts: Vec<VolumeMount>,
    pub env: Option<Vec<Env>>,
    pub resource: Option<ResourceRequirements>,
    pub termination_mssage_path: Option<String>,
    pub termination_message_policy: Option<String>,
    pub tty: Option<bool>
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    pub api_groups: Vec<String>,
    pub resource_names: Vec<String>,
    pub resources: Vec<String>,
    pub verbs: Vec<String>
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPortSpec {
    pub container_port: u16,
    pub name: Option<String>,
    pub protocol: Option<String>, // TODO: This should be enum
}

impl ContainerPortSpec {
    pub fn new<T: Into<String>>(container_port: u16, name: T) -> Self {
        ContainerPortSpec {
            container_port,
            name: Some(name.into()),
            protocol: None,
        }
    }
}



#[derive(Deserialize, Serialize, Default, Debug, Clone)]
pub struct VolumeSpec {
    pub name: String,
    pub secret: Option<SecretVolumeSpec>,
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    pub mount_path: String,
    pub mount_propagation: Option<String>,
    pub name: String,
    pub read_only: Option<bool>,
    pub sub_path: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretVolumeSpec {
    pub default_mode: u16,
    pub secret_name: String,
    pub optional: Option<bool>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimVolumeSource {
    claim_name: String,
    read_only: bool,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodStatus {
    pub phase: String,
    #[serde(rename = "hostIP")]
    pub host_ip: String,
    #[serde(rename = "podIP")]
    pub pod_ip: String,
    pub start_time: String,
    pub container_statuses: Vec<ContainerStatus>,
}

impl Status for PodStatus{}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatus {
    pub name: String,
    pub state: ContainerState,
    pub ready: bool,
    pub restart_count: u8,
    pub image: String,
    #[serde(rename = "imageID")]
    pub image_id: String,
    #[serde(rename = "containerID")]
    pub container_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerState {
    pub running: ContainerStateRunning,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStateRunning {
    pub started_at: String,
}

//
// Test Cases
//
#[cfg(test)]
mod test {

    use k8_obj_metadata::item_uri;
    use k8_obj_metadata::items_uri;
    use k8_obj_metadata::DEFAULT_NS;
    use crate::pod::PodSpec;

    #[test]
    fn test_pod_item_uri() {
        let uri = item_uri::<PodSpec>("https://localhost", "test", DEFAULT_NS, None);
        assert_eq!(
            uri,
            "https://localhost/api/v1/namespaces/default/pods/test"
        );
    }

    #[test]
    fn test_pod_items_uri() {
        let uri = items_uri::<PodSpec>("https://localhost", DEFAULT_NS, None);
        assert_eq!(
            uri,
            "https://localhost/api/v1/namespaces/default/pods"
        );
    }


}
