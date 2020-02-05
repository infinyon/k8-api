use serde::Deserialize;
use serde::Serialize;

use k8_metadata_core::Crd;
use k8_metadata_core::CrdNames;
use k8_metadata_core::Spec;
use k8_metadata_core::Status;

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

    fn metadata() -> &'static Crd {
        &SECRET_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretSpec {}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SecretStatus {}

impl Status for SecretStatus{}
