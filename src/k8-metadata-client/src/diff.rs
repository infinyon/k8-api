use serde::Serialize;

use k8_obj_metadata::Crd;
use k8_obj_metadata::K8Obj;
use k8_obj_metadata::Spec;

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

/// used for comparing spec,
#[derive(Serialize, Debug, Clone)]
pub struct DiffSpec<S> {
    spec: S,
}

impl<S> DiffSpec<S>
where
    S: Serialize,
{
    pub fn from(spec: S) -> Self {
        DiffSpec { spec }
    }
}
