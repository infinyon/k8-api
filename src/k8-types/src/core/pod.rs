use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as DynamicObject;

use crate::Crd;
use crate::CrdNames;
use crate::DefaultHeader;
use crate::Env;
use crate::Spec;
use crate::Status;

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
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &POD_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PodSpec {
    pub volumes: Vec<VolumeSpec>,
    pub containers: Vec<ContainerSpec>,
    pub restart_policy: Option<PodRestartPolicy>,
    pub service_account_name: Option<String>,
    pub service_account: Option<String>,
    pub node_name: Option<String>,
    pub termination_grace_period_seconds: Option<u16>,
    pub dns_policy: Option<String>,
    pub security_context: Option<PodSecurityContext>,
    pub scheduler_name: Option<String>,
    pub node_selector: Option<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum PodRestartPolicy {
    Always,
    Never,
    OnFailure,
}
impl Default for PodRestartPolicy {
    fn default() -> Self {
        Self::Always // https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#restart-policy
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PodSecurityContext {
    pub fs_group: Option<u32>,
    pub run_as_group: Option<u32>,
    pub run_as_non_root: Option<bool>,
    pub run_as_user: Option<u32>,
    pub sysctls: Vec<Sysctl>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sysctl {
    pub name: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ContainerSpec {
    pub name: String,
    pub args: Vec<String>,
    pub command: Vec<String>,
    pub ports: Vec<ContainerPortSpec>,
    pub image: Option<String>,
    pub image_pull_policy: Option<ImagePullPolicy>, // TODO: should be enum
    pub volume_mounts: Vec<VolumeMount>,
    pub env: Vec<Env>,
    pub resources: Option<ResourceRequirements>,
    pub termination_mssage_path: Option<String>,
    pub termination_message_policy: Option<String>,
    pub tty: Option<bool>,
    pub liveness_probe: Option<Probe>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub enum ImagePullPolicy {
    Always,
    Never,
    IfNotPresent,
}

impl Default for ImagePullPolicy {
    fn default() -> Self {
        Self::Always // https://kubernetes.io/docs/concepts/containers/images/#updating-images
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Probe {
    pub exec: Option<ExecAction>,
    pub failure_threshold: Option<u32>,
    pub initial_delay_seconds: Option<u32>,
    pub period_seconds: Option<u32>,
    pub success_threshold: Option<u32>,
    pub tcp_socket: Option<TcpSocketAction>,
    pub timeout_seconds: Option<u32>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ExecAction {
    pub command: Vec<String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct TcpSocketAction {
    pub host: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ResourceRequirements {
    pub limits: DynamicObject,
    pub requests: DynamicObject,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
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

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSpec {
    pub name: String,
    pub secret: Option<SecretVolumeSpec>,
    pub config_map: Option<ConfigMapVolumeSource>,
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct VolumeMount {
    pub mount_path: String,
    pub mount_propagation: Option<String>,
    pub name: String,
    pub read_only: Option<bool>,
    pub sub_path: Option<String>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SecretVolumeSpec {
    pub default_mode: u16,
    pub secret_name: String,
    pub optional: Option<bool>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapVolumeSource {
    pub default_mode: Option<i32>,
    pub items: Option<Vec<KeyToPath>>,
    pub name: Option<String>,
    pub optional: Option<bool>,
}
#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyToPath {
    pub key: String,
    pub mode: Option<i32>,
    pub path: String,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PersistentVolumeClaimVolumeSource {
    claim_name: String,
    read_only: Option<bool>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PodStatus {
    pub phase: String,
    #[serde(rename = "hostIP")]
    pub host_ip: String,
    #[serde(rename = "podIP")]
    pub pod_ip: Option<String>,
    pub start_time: String,
    pub container_statuses: Vec<ContainerStatus>,
}

impl Status for PodStatus {}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatus {
    pub name: String,
    pub state: ContainerState,
    pub ready: bool,
    pub restart_count: i32,
    pub image: String,
    #[serde(rename = "imageID")]
    pub image_id: String,
    #[serde(rename = "containerID")]
    pub container_id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerState {
    pub running: Option<ContainerStateRunning>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStateRunning {
    pub started_at: String,
}
