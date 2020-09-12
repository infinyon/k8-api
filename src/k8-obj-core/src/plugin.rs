use serde::Deserialize;
use serde::Serialize;

use k8_obj_metadata::Crd;
use k8_obj_metadata::CrdNames;
use k8_obj_metadata::DefaultHeader;
use k8_obj_metadata::Spec;
use k8_obj_metadata::Status;

const CREDENTIAL_API: Crd = Crd {
    group: "client.authentication.k8s.io",
    version: "v1",
    names: CrdNames {
        kind: "ExecCrendetial",
        plural: "credentials",
        singular: "credential",
    },
};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecCredentialSpec {}

impl Spec for ExecCredentialSpec {
    type Status = ExecCredentialStatus;
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &CREDENTIAL_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExecCredentialStatus {
    pub expiration_timestamp: String,
    pub token: String,
}

impl Status for ExecCredentialStatus {}
