use serde::Deserialize;
use serde::Serialize;

use crate::core::pod::PodSpec;
use crate::{Crd, CrdNames, DefaultHeader, Int32OrString, LabelSelector, Spec, Status, TemplateSpec};
const DEPLOYMENT_API: Crd = Crd {
    group: "apps",
    version: "v1",
    names: CrdNames {
        kind: "Deployment",
        plural: "deployments",
        singular: "deployment",
    },
};

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DeploymentStrategy {
    pub rolling_update: Option<RollingUpdateDeployment>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RollingUpdateDeployment {
    pub max_surge: Option<Int32OrString>,
    pub max_unavailable: Option<Int32OrString>,
}

impl Spec for DeploymentSpec {
    type Status = DeploymentStatus;
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &DEPLOYMENT_API
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentStatus {
    pub available_replicas: Option<i32>,
    pub collision_count: Option<i32>,
    #[serde(default = "Vec::new")]
    pub conditions: Vec<DeploymentCondition>,
    pub observed_generation: Option<i64>,
    pub ready_replicas: Option<i32>,
    pub replicas: Option<i32>,
    pub unavailable_replicas: Option<i32>,
    pub updated_replicas: Option<i32>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentCondition {
    pub last_transition_time: Option<String>,
    pub last_update_time: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub status: String,
    #[serde(rename = "type")]
    pub type_: String,
}

impl Status for DeploymentStatus {}
