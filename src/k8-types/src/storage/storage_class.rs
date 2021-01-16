use serde::Deserialize;
use serde::Serialize;

use crate::{Crd, CrdNames, Header, Spec, Status};

const STORAGE_API: Crd = Crd {
    group: "storage.k8s.io",
    version: "v1",
    names: CrdNames {
        kind: "StorageClass",
        plural: "storageclasses",
        singular: "storageclass",
    },
};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageClassSpec {}

impl Spec for StorageClassSpec {
    type Status = StorageClassStatus;
    type Header = StorageClassHeader;
    const NAME_SPACED: bool = false;

    fn metadata() -> &'static Crd {
        &STORAGE_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageClassHeader {
    pub allow_volume_expansion: Option<bool>,
    pub provisioner: String,
    pub reclaim_policy: String,
    pub volume_binding_mode: String,
}

impl Header for StorageClassHeader {}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StorageClassStatus {}

impl Status for StorageClassStatus {}
