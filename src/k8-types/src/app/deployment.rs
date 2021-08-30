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
