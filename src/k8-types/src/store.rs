use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use std::io::Error as IoError;

use crate::K8Obj;
use crate::ObjectMeta;
use crate::Spec;

// Spec that can store in meta store
pub trait StoreSpec: Sized + Default + Debug + Clone {
    type K8Spec: Spec;
    type Status: Sized + Clone + Default + Debug;
    type Key: Ord + Clone + Debug + ToString;
    type Owner: StoreSpec;

    const LABEL: &'static str;

    // convert kubernetes objects into KV value
    fn convert_from_k8(k8_obj: K8Obj<Self::K8Spec>) -> Result<Option<MetaItem<Self>>, IoError>;
}

/// Metadata object. Used to be KVObject int sc-core
#[derive(Debug, Clone, PartialEq)]
pub struct MetaItem<S>
where
    S: StoreSpec,
{
    pub spec: S,
    pub status: S::Status,
    pub key: S::Key,
    pub ctx: MetaItemContext,
}

impl<S> MetaItem<S>
where
    S: StoreSpec,
{
    pub fn new<J>(key: J, spec: S, status: S::Status, ctx: MetaItemContext) -> Self
    where
        J: Into<S::Key>,
    {
        Self {
            key: key.into(),
            spec,
            status,
            ctx,
        }
    }

    pub fn with_ctx(mut self, ctx: MetaItemContext) -> Self {
        self.ctx = ctx;
        self
    }

    pub fn key(&self) -> &S::Key {
        &self.key
    }

    pub fn key_owned(&self) -> S::Key {
        self.key.clone()
    }

    pub fn my_key(self) -> S::Key {
        self.key
    }

    pub fn spec(&self) -> &S {
        &self.spec
    }
    pub fn status(&self) -> &S::Status {
        &self.status
    }

    pub fn set_status(&mut self, status: S::Status) {
        self.status = status;
    }

    pub fn ctx(&self) -> &MetaItemContext {
        &self.ctx
    }

    pub fn set_ctx(&mut self, ctx: MetaItemContext) {
        self.ctx = ctx;
    }

    pub fn parts(self) -> (S::Key, S, MetaItemContext) {
        (self.key, self.spec, self.ctx)
    }

    pub fn is_owned(&self, uid: &str) -> bool {
        match &self.ctx.parent_ctx {
            Some(parent) => parent.uid == uid,
            None => false,
        }
    }

    pub fn with_spec<J>(key: J, spec: S) -> Self
    where
        J: Into<S::Key>,
    {
        Self::new(
            key.into(),
            spec,
            S::Status::default(),
            MetaItemContext::default(),
        )
    }
}

impl<S> fmt::Display for MetaItem<S>
where
    S: StoreSpec,
    S::Key: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MetaItem {} key: {}", S::LABEL, self.key())
    }
}

impl<S> From<MetaItem<S>> for (S::Key, S, S::Status)
where
    S: StoreSpec,
{
    fn from(val: MetaItem<S>) -> Self {
        (val.key, val.spec, val.status)
    }
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct MetaItemContext {
    pub item_ctx: Option<ObjectMeta>,
    pub parent_ctx: Option<ObjectMeta>,
}

impl MetaItemContext {
    pub fn with_ctx(mut self, ctx: ObjectMeta) -> Self {
        self.item_ctx = Some(ctx);
        self
    }

    pub fn with_parent_ctx(mut self, ctx: ObjectMeta) -> Self {
        self.parent_ctx = Some(ctx);
        self
    }

    pub fn make_parent_ctx(&self) -> Self {
        if self.item_ctx.is_some() {
            Self::default().with_parent_ctx(self.item_ctx.as_ref().unwrap().clone())
        } else {
            Self::default()
        }
    }
}

/// define default store spec assuming key is string
#[macro_export]
macro_rules! default_store_spec {
    ($spec:ident,$status:ident,$name:expr) => {
        impl crate::store::StoreSpec for $spec {
            const LABEL: &'static str = $name;

            type K8Spec = Self;
            type Status = $status;
            type Key = String;
            type Owner = Self;

            fn convert_from_k8(
                k8_obj: crate::K8Obj<Self::K8Spec>,
            ) -> Result<Option<crate::store::MetaItem<Self>>, std::io::Error> {
                let ctx =
                    crate::store::MetaItemContext::default().with_ctx(k8_obj.metadata.clone());
                Ok(Some(crate::store::MetaItem::new(
                    k8_obj.metadata.name,
                    k8_obj.spec,
                    k8_obj.status,
                    ctx,
                )))
            }
        }
    };
}
