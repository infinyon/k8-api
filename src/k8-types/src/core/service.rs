use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;

use crate::default_store_spec;
use crate::Crd;
use crate::CrdNames;
use crate::DefaultHeader;
use crate::Spec;
use crate::Status;

const SERVICE_API: Crd = Crd {
    group: "core",
    version: "v1",
    names: CrdNames {
        kind: "Service",
        plural: "services",
        singular: "service",
    },
};

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct ServiceSpec {
    #[serde(rename = "clusterIP")]
    pub cluster_ip: String,
    #[serde(rename = "externalIPs")]
    pub external_ips: Vec<String>,
    #[serde(rename = "loadBalancerIP")]
    pub load_balancer_ip: Option<String>,
    pub r#type: Option<LoadBalancerType>,
    pub external_name: Option<String>,
    pub external_traffic_policy: Option<ExternalTrafficPolicy>,
    pub ports: Vec<ServicePort>,
    pub selector: Option<HashMap<String, String>>,
}

impl Spec for ServiceSpec {
    type Status = ServiceStatus;
    type Header = DefaultHeader;

    fn metadata() -> &'static Crd {
        &SERVICE_API
    }

    fn make_same(&mut self, other: &Self) {
        if other.cluster_ip.is_empty() {
            "".clone_into(&mut self.cluster_ip);
        }
    }
}

default_store_spec!(ServiceSpec, ServiceStatus, "Service");

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServicePort {
    pub name: Option<String>,
    pub node_port: Option<u16>,
    pub port: u16,
    pub target_port: Option<TargetPort>,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[serde(untagged)]
pub enum TargetPort {
    Number(u16),
    Name(String),
}

impl std::fmt::Display for TargetPort {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Number(value) => write!(f, "{}", value),
            Self::Name(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct ServiceStatus {
    pub load_balancer: LoadBalancerStatus,
}

impl Status for ServiceStatus {}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
pub enum ExternalTrafficPolicy {
    Local,
    Cluster,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
pub enum LoadBalancerType {
    ExternalName,
    #[allow(clippy::upper_case_acronyms)]
    ClusterIP,
    NodePort,
    LoadBalancer,
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct LoadBalancerStatus {
    pub ingress: Vec<LoadBalancerIngress>,
}

impl LoadBalancerStatus {
    /// find any ip or host
    pub fn find_any_ip_or_host(&self) -> Option<&str> {
        self.ingress.iter().find_map(|ingress| ingress.host_or_ip())
    }
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LoadBalancerIngress {
    pub hostname: Option<String>,
    pub ip: Option<String>,
}

impl LoadBalancerIngress {
    /// return either host or ip
    pub fn host_or_ip(&self) -> Option<&str> {
        if let Some(host) = &self.hostname {
            Some(host)
        } else if let Some(ip) = &self.ip {
            Some(ip)
        } else {
            None
        }
    }
}
