use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::Crd;
use crate::CrdNames;
use crate::Header;
use crate::Spec;
use crate::Status;

//
// ConfigMap Object
const CONFIG_MAP_API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "ConfigMap",
        plural: "configmaps",
        singular: "configmap",
    },
};

impl Spec for ConfigMapSpec {
    type Status = ConfigMapStatus;
    type Header = ConfigMapHeader;

    fn metadata() -> &'static Crd {
        &CONFIG_MAP_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapSpec {}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapStatus {}

impl Status for ConfigMapStatus {}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapHeader {
    #[serde(default)]
    pub data: BTreeMap<String, String>,
}

impl Header for ConfigMapHeader {}
