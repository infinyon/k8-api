use serde::Serialize;

use crate::k8_types::{Crd, K8Obj, Spec};

#[derive(Debug)]
pub enum ApplyResult<S>
where
    S: Spec,
{
    None,
    Created(K8Obj<S>),
    Patched(K8Obj<S>),
}

#[allow(dead_code)]
pub enum PatchMergeType {
    Json,
    JsonMerge,
    StrategicMerge, // for aggegration API
}

impl PatchMergeType {
    pub fn for_spec(crd: &Crd) -> Self {
        match crd.group {
            "core" => PatchMergeType::StrategicMerge,
            "apps" => PatchMergeType::StrategicMerge,
            _ => PatchMergeType::JsonMerge,
        }
    }

    pub fn content_type(&self) -> &'static str {
        match self {
            PatchMergeType::Json => "application/json-patch+json",
            PatchMergeType::JsonMerge => "application/merge-patch+json",
            PatchMergeType::StrategicMerge => "application/strategic-merge-patch+json",
        }
    }
}

/// used for comparing k8 objects,
#[derive(Serialize, Debug, Clone)]
pub struct DiffableK8Obj<M, S> {
    metadata: M,
    spec: S,
}

impl<M, S> DiffableK8Obj<M, S>
where
    M: Serialize,
    S: Serialize,
{
    pub fn new(metadata: M, spec: S) -> Self {
        Self { metadata, spec }
    }
}
