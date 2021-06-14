use serde::Deserialize;
use serde::Serialize;

use crate::core::pod::PodSpec;
use crate::{Crd, CrdNames, DefaultHeader, LabelSelector, Spec, Status, TemplateSpec};

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct JobSpec {
    pub template: TemplateSpec<PodSpec>,
    pub backoff_limit: Option<usize>,
    pub active_deadline_seconds: Option<usize>,
    pub paralellism: Option<usize>,
    pub completions: Option<usize>,
    pub completion_mode: Option<CompletionMode>,
    pub suspend: Option<bool>,
    pub selector: Option<LabelSelector>,
    pub manual_selector: Option<bool>,
    pub ttl_seconds_after_finished: Option<usize>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum CompletionMode {
    Indexed,
    NonIndexed,
}

impl Spec for JobSpec {
    type Status = JobStatus;
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &API
    }
}

const API: Crd = Crd {
    group: "batch",
    version: "v1",
    names: CrdNames {
        kind: "Job",
        plural: "jobs",
        singular: "job",
    },
};

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct JobStatus {
    active: usize,
    completed_indices: Option<String>,
    completion_time: Option<String>,
    failed: usize,
    job_condition: Vec<JobCondition>,
    start_time: Option<String>,
    succeeded: usize,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct JobCondition {
    last_probe_time: String,
    last_transition_time: String,
    message: String,
    reason: String,
    status: JobConditionStatus,
    #[serde(rename = "type")]
    _type: JobType,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum JobConditionStatus {
    True,
    False,
    Unknown,
}
#[derive(Deserialize, Serialize, Debug, Clone)]

pub enum JobType {
    Completed,
    Failed,
}

impl Status for JobStatus {}
