use serde::Deserialize;
use serde::Serialize;

use k8_obj_metadata::default_store_spec;
use k8_obj_metadata::Crd;
use k8_obj_metadata::CrdNames;
use k8_obj_metadata::DefaultHeader;
use k8_obj_metadata::Spec;
use k8_obj_metadata::Status;

const API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "Namespace",
        plural: "namespaces",
        singular: "namespace",
    },
};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NamespaceSpec {}

impl Spec for NamespaceSpec {
    type Status = NamespaceStatus;
    type Header = DefaultHeader;
    const NAME_SPACED: bool = false;

    fn metadata() -> &'static Crd {
        &API
    }
}

default_store_spec!(NamespaceSpec, NamespaceStatus, "Namespace");

#[derive(Deserialize, Serialize, PartialEq, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct NamespaceStatus {
    pub phase: String,
}

impl Status for NamespaceStatus {}
