use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Affinity {
    pub node_affinity: Option<NodeAffinity>,
    pub pod_affinity: Option<PodAffinity>,
    pub pod_anti_affinity: Option<PodAffinity>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeAffinity {
    #[serde(rename = "requiredDuringSchedulingIgnoredDuringExecution")]
    pub required: Option<NodeSelector>,
    #[serde(rename = "preferredDuringSchedulingIgnoredDuringExecution")]
    pub preferred: Option<Vec<PreferredSchedulingTerm>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectorTerm {
    pub match_expressions: Option<Vec<SelectorRequirement>>,
    pub match_fields: Option<Vec<SelectorRequirement>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SelectorRequirement {
    pub key: String,
    pub operator: SelectorOperator,
    pub values: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum SelectorOperator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
    Gt,
    Lt,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PreferredSchedulingTerm {
    pub weight: i32,
    pub preference: NodeSelectorTerm,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PodAffinity {
    #[serde(rename = "requiredDuringSchedulingIgnoredDuringExecution")]
    pub required: Option<Vec<PodAffinityTerm>>,
    #[serde(rename = "preferredDuringSchedulingIgnoredDuringExecution")]
    pub preferred: Option<Vec<WeightedPodAffinityTerm>>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeightedPodAffinityTerm {
    pub weight: i32,
    pub pod_affinity_term: PodAffinityTerm,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PodAffinityTerm {
    pub label_selector: Option<LabelSelector>,
    pub namespace_selector: Option<LabelSelector>,
    #[serde(default)]
    pub namespaces: Option<Vec<String>>,
    pub topology_key: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    pub match_expressions: Option<Vec<SelectorRequirement>>,
    pub match_labels: Option<HashMap<String, String>>,
}
