use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::Crd;
use crate::CrdNames;
use crate::Header;
use crate::Spec;
use crate::Status;

//
// Secret Object
const SECRET_API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "Secret",
        plural: "secrets",
        singular: "secret",
    },
};

impl Spec for SecretSpec {
    type Status = SecretStatus;
    type Header = SecretHeader;

    fn metadata() -> &'static Crd {
        &SECRET_API
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretSpec {}

#[derive(Deserialize, Serialize, Default, Eq, PartialEq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {}

impl Status for SecretStatus {}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretHeader {
    #[serde(default)]
    pub data: BTreeMap<String, String>,
    #[serde(rename = "type")]
    pub ty: String,
}

impl Header for SecretHeader {}
