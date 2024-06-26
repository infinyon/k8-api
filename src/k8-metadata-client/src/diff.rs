use serde::Serialize;

use k8_types::{Crd, K8Obj, Spec};

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
    Apply,
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
            PatchMergeType::Apply => "application/apply-patch+yaml",
        }
    }
}

/// used for comparing k8 objects,
#[derive(Serialize, Debug, Clone)]
pub struct DiffableK8Obj<M, S, H> {
    metadata: M,
    spec: S,
    #[serde(flatten)]
    header: H,
}

impl<M, S, H> DiffableK8Obj<M, S, H>
where
    M: Serialize,
    S: Serialize,
    H: Serialize,
{
    pub fn new(metadata: M, spec: S, header: H) -> Self {
        Self {
            metadata,
            spec,
            header,
        }
    }
}
