use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::de::Deserializer;
use serde::Deserialize;
use serde::Serialize;

use crate::Spec;

pub const DEFAULT_NS: &'static str = "default";
pub const TYPE_OPAQUE: &'static str = "Opaque";

pub trait K8Meta {
    /// resource name
    fn name(&self) -> &str;

    /// namespace
    fn namespace(&self) -> &str;
}

pub trait LabelProvider: Sized {
    fn set_label_map(self, labels: HashMap<String, String>) -> Self;

    /// helper for setting list of labels
    fn set_labels<T: ToString>(self, labels: Vec<(T, T)>) -> Self {
        let mut label_map = HashMap::new();
        for (key, value) in labels {
            label_map.insert(key.to_string(), value.to_string());
        }
        self.set_label_map(label_map)
    }
}

/// metadata associated with object when returned
/// here name and namespace must be populated
#[derive(Deserialize, Serialize, PartialEq, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct ObjectMeta {
    // mandatory fields
    pub name: String,
    pub namespace: String,
    pub uid: String,
    pub self_link: String,
    pub creation_timestamp: String,
    pub generation: Option<i32>,
    pub resource_version: String,
    // optional
    pub cluster_name: Option<String>,
    pub deletion_timestamp: Option<String>,
    pub deletion_grace_period_seconds: Option<u32>,
    pub labels: HashMap<String, String>,
    pub owner_references: Vec<OwnerReferences>,
    pub annotations: HashMap<String, String>,
}

impl LabelProvider for ObjectMeta {
    fn set_label_map(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

impl K8Meta for ObjectMeta {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl ObjectMeta {
    pub fn new<S>(name: S, name_space: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            namespace: name_space.into(),
            ..Default::default()
        }
    }

    /// provide builder pattern setter
    pub fn set_labels<T: Into<String>>(mut self, labels: Vec<(T, T)>) -> Self {
        let mut label_map = HashMap::new();
        for (key, value) in labels {
            label_map.insert(key.into(), value.into());
        }
        self.labels = label_map;
        self
    }

    /// create with name and default namespace
    pub fn named<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// create owner references point to this metadata
    /// if name or uid doesn't exists return none
    pub fn make_owner_reference<S: Spec>(&self) -> OwnerReferences {
        OwnerReferences {
            api_version: S::api_version(),
            kind: S::kind(),
            name: self.name.clone(),
            uid: self.uid.clone(),
            controller: Some(true),
            ..Default::default()
        }
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// create child references that points to this
    pub fn make_child_input_metadata<S: Spec>(&self, childname: String) -> InputObjectMeta {
        let mut owner_refs: Vec<OwnerReferences> = vec![];
        owner_refs.push(self.make_owner_reference::<S>());

        InputObjectMeta {
            name: childname,
            namespace: self.namespace().to_owned(),
            owner_references: owner_refs,
            ..Default::default()
        }
    }

    pub fn as_input(&self) -> InputObjectMeta {
        InputObjectMeta {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            ..Default::default()
        }
    }

    pub fn as_item(&self) -> ItemMeta {
        ItemMeta {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
        }
    }

    pub fn as_update(&self) -> UpdateItemMeta {
        UpdateItemMeta {
            name: self.name.clone(),
            namespace: self.namespace.clone(),
            resource_version: self.resource_version.clone(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InputObjectMeta {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub namespace: String,
    pub owner_references: Vec<OwnerReferences>,
}

impl LabelProvider for InputObjectMeta {
    fn set_label_map(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

impl fmt::Display for InputObjectMeta {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.namespace)
    }
}

impl K8Meta for InputObjectMeta {
    fn name(&self) -> &str {
        &self.name
    }

    fn namespace(&self) -> &str {
        &self.namespace
    }
}

impl InputObjectMeta {
    // shorthand to create just with name and metadata
    pub fn named<S: Into<String>>(name: S, namespace: S) -> Self {
        InputObjectMeta {
            name: name.into(),
            namespace: namespace.into(),
            ..Default::default()
        }
    }
}

impl From<ObjectMeta> for InputObjectMeta {
    fn from(meta: ObjectMeta) -> Self {
        Self {
            name: meta.name,
            namespace: meta.namespace,
            ..Default::default()
        }
    }
}

/// used for retrieving,updating and deleting item
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ItemMeta {
    pub name: String,
    pub namespace: String,
}

impl From<ObjectMeta> for ItemMeta {
    fn from(meta: ObjectMeta) -> Self {
        Self {
            name: meta.name,
            namespace: meta.namespace,
        }
    }
}

/// used for updating item
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateItemMeta {
    pub name: String,
    pub namespace: String,
    pub resource_version: String,
}

impl From<ObjectMeta> for UpdateItemMeta {
    fn from(meta: ObjectMeta) -> Self {
        Self {
            name: meta.name,
            namespace: meta.namespace,
            resource_version: meta.resource_version,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwnerReferences {
    pub api_version: String,
    pub block_owner_deletion: Option<bool>,
    pub controller: Option<bool>,
    pub kind: String,
    pub name: String,
    pub uid: String,
}

#[derive(Deserialize, Debug, Eq, PartialEq, Clone)]
pub enum StatusEnum {
    SUCCESS,
    FAILURE,
}

impl DeserializeWith for StatusEnum {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "Success" => Ok(StatusEnum::SUCCESS),
            "Failure" => Ok(StatusEnum::FAILURE),
            _ => Err(serde::de::Error::custom(
                "error trying to deserialize status type",
            )),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct K8Status {
    pub api_version: String,
    pub code: Option<u16>,
    pub details: Option<StatusDetails>,
    pub kind: String,
    pub message: Option<String>,
    pub reason: Option<String>,
    #[serde(deserialize_with = "StatusEnum::deserialize_with")]
    pub status: StatusEnum,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StatusDetails {
    pub name: String,
    pub group: Option<String>,
    pub kind: String,
    pub uid: String,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(bound(serialize = "S: Serialize"))]
#[serde(bound(deserialize = "S: DeserializeOwned"))]
pub struct K8Obj<S>
where
    S: Spec,
{
    #[serde(default = "S::api_version")]
    pub api_version: String,
    #[serde(default = "S::kind")]
    pub kind: String,
    #[serde(default)]
    pub metadata: ObjectMeta,
    #[serde(default)]
    pub spec: S,
    #[serde(flatten)]
    pub header: S::Header,
    #[serde(default)]
    pub status: S::Status,
}

impl<S> K8Obj<S>
where
    S: Spec,
{
    #[allow(dead_code)]
    pub fn new<N>(name: N, spec: S) -> Self
    where
        N: Into<String>,
    {
        Self {
            api_version: S::api_version(),
            kind: S::kind(),
            metadata: ObjectMeta::named(name),
            spec,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn set_status(mut self, status: S::Status) -> Self {
        self.status = status;
        self
    }

    pub fn as_status_update(&self, status: S::Status) -> UpdateK8ObjStatus<S> {
        UpdateK8ObjStatus {
            api_version: S::api_version(),
            kind: S::kind(),
            metadata: self.metadata.as_update(),
            status,
            ..Default::default()
        }
    }
}

impl<S> K8Obj<S>
where
    S: Spec,
{
    pub fn as_input(&self) -> InputK8Obj<S> {
        K8SpecObj {
            api_version: self.api_version.clone(),
            kind: self.kind.clone(),
            metadata: self.metadata.as_input(),
            spec: self.spec.clone(),
            ..Default::default()
        }
    }
}

/// For creating, only need spec
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct K8SpecObj<S, M> {
    pub api_version: String,
    pub kind: String,
    pub metadata: M,
    pub spec: S,
    #[serde(default)]
    pub data: BTreeMap<String, String>,
}

impl<S, M> K8SpecObj<S, M>
where
    S: Spec,
{
    pub fn new(spec: S, metadata: M) -> Self
    where
        M: Default,
    {
        Self {
            api_version: S::api_version(),
            kind: S::kind(),
            metadata,
            spec,
            ..Default::default()
        }
    }
}

pub type InputK8Obj<S> = K8SpecObj<S, InputObjectMeta>;
pub type UpdateK8Obj<S> = K8SpecObj<S, ItemMeta>;

/// Used for updating k8obj
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateK8ObjStatus<S>
where
    S: Spec,
{
    pub api_version: String,
    pub kind: String,
    pub metadata: UpdateItemMeta,
    pub status: S::Status,
    pub data: PhantomData<S>,
}

impl<S> UpdateK8ObjStatus<S>
where
    S: Spec,
{
    pub fn new(status: S::Status, metadata: UpdateItemMeta) -> Self {
        Self {
            api_version: S::api_version(),
            kind: S::kind(),
            metadata,
            status,
            ..Default::default()
        }
    }
}

impl<S> From<UpdateK8Obj<S>> for InputK8Obj<S>
where
    S: Default,
{
    fn from(update: UpdateK8Obj<S>) -> Self {
        Self {
            api_version: update.api_version,
            kind: update.kind,
            metadata: update.metadata.into(),
            spec: update.spec,
            ..Default::default()
        }
    }
}

impl From<ItemMeta> for InputObjectMeta {
    fn from(update: ItemMeta) -> Self {
        Self {
            name: update.name,
            namespace: update.namespace,
            ..Default::default()
        }
    }
}

/// name is optional for template
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct TemplateMeta {
    pub name: Option<String>,
    pub creation_timestamp: Option<String>,
    pub labels: HashMap<String, String>,
}

impl LabelProvider for TemplateMeta {
    fn set_label_map(mut self, labels: HashMap<String, String>) -> Self {
        self.labels = labels;
        self
    }
}

impl TemplateMeta {
    /// create with name and default namespace
    pub fn named<S>(name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSpec<S> {
    pub metadata: Option<TemplateMeta>,
    pub spec: S,
}

impl<S> TemplateSpec<S> {
    pub fn new(spec: S) -> Self {
        TemplateSpec {
            metadata: None,
            spec,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(bound(serialize = "K8Obj<S>: Serialize"))]
#[serde(bound(deserialize = "K8Obj<S>: DeserializeOwned"))]
pub struct K8List<S>
where
    S: Spec,
{
    pub api_version: String,
    pub items: Vec<K8Obj<S>>,
    pub kind: String,
    pub metadata: ListMetadata,
}

impl<S> K8List<S>
where
    S: Spec,
{
    #[allow(dead_code)]
    pub fn new() -> Self {
        K8List {
            api_version: S::api_version(),
            items: vec![],
            kind: S::kind(),
            metadata: ListMetadata {
                _continue: None,
                resource_version: S::api_version(),
                self_link: "".to_owned(),
            },
        }
    }
}

pub trait DeserializeWith: Sized {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "object")]
#[serde(bound(serialize = "K8Obj<S>: Serialize"))]
#[serde(bound(deserialize = "K8Obj<S>: DeserializeOwned"))]
pub enum K8Watch<S>
where
    S: Spec,
{
    ADDED(K8Obj<S>),
    MODIFIED(K8Obj<S>),
    DELETED(K8Obj<S>),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ListMetadata {
    pub _continue: Option<String>,
    pub resource_version: String,
    pub self_link: String,
}

#[derive(Deserialize, Serialize, Default, Debug, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LabelSelector {
    pub match_labels: HashMap<String, String>,
}

impl LabelSelector {
    pub fn new_labels<T: Into<String>>(labels: Vec<(T, T)>) -> Self {
        let mut match_labels = HashMap::new();
        for (key, value) in labels {
            match_labels.insert(key.into(), value.into());
        }
        LabelSelector { match_labels }
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Env {
    pub name: String,
    pub value: Option<String>,
    pub value_from: Option<EnvVarSource>,
}

impl Env {
    pub fn key_value<T: Into<String>>(name: T, value: T) -> Self {
        Env {
            name: name.into(),
            value: Some(value.into()),
            value_from: None,
        }
    }

    pub fn key_field_ref<T: Into<String>>(name: T, field_path: T) -> Self {
        Env {
            name: name.into(),
            value: None,
            value_from: Some(EnvVarSource {
                field_ref: Some(ObjectFieldSelector {
                    field_path: field_path.into(),
                }),
            }),
        }
    }
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSource {
    field_ref: Option<ObjectFieldSelector>,
}

#[derive(Deserialize, Serialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ObjectFieldSelector {
    pub field_path: String,
}

#[cfg(test)]
mod test {

    use super::Env;
    use super::ObjectMeta;

    #[test]
    fn test_metadata_label() {
        let metadata =
            ObjectMeta::default().set_labels(vec![("app".to_owned(), "test".to_owned())]);

        let maps = metadata.labels;
        assert_eq!(maps.len(), 1);
        assert_eq!(maps.get("app").unwrap(), "test");
    }

    #[test]
    fn test_env() {
        let env = Env::key_value("lang", "english");
        assert_eq!(env.name, "lang");
        assert_eq!(env.value, Some("english".to_owned()));
    }
}
