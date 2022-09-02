use serde::Deserialize;
use serde::Serialize;

use crate::Crd;
use crate::CrdNames;
use crate::DefaultHeader;
use crate::Spec;
use crate::Status;

const NODE_API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "Node",
        plural: "nodes",
        singular: "node",
    },
};

impl Spec for NodeSpec {
    type Status = NodeStatus;
    type Header = DefaultHeader;
    const NAME_SPACED: bool = false;

    fn metadata() -> &'static Crd {
        &NODE_API
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeSpec {
    #[serde(rename = "providerID")]
    pub provider_id: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeStatus {
    pub addresses: Vec<NodeAddress>,
    //phase: String,
    //node_info: String,
    //volumes_attached: Vec<String>,
    //volumesAttached: AttachedVolume
}

impl Status for NodeStatus {}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeList {}

#[derive(Deserialize, Serialize, Debug, Default, Clone, Eq, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeAddress {
    pub address: String,
    pub r#type: String,
}

//#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
//#[serde(rename_all = "camelCase", default)]
//pub enum NodeAddressType {
//    Hostname,
//    #[serde(rename = "InternalIP")]
//    InternalIp,
//    #[serde(rename = "ExternalIP")]
//    ExternalIp
//}
