use serde::Deserialize;
use serde::Serialize;

use crate::default_store_spec;
use crate::Crd;
use crate::CrdNames;
use crate::DefaultHeader;
use crate::Spec;
use crate::Status;

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

#[derive(Deserialize, Serialize, Eq, PartialEq, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct NamespaceStatus {
    pub phase: String,
}

impl Status for NamespaceStatus {}
