use serde::Deserialize;
use serde::Serialize;

use crate::Crd;
use crate::CrdNames;
use crate::DefaultHeader;
use crate::Spec;
use crate::Status;

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
