
use std::fmt::Debug;
use std::fmt;
use std::fmt::Display;
use std::io::Error as IoError;

use crate::Spec;
use crate::ObjectMeta;
use crate::K8Obj;

/// Spec that can be stored with key
pub trait SpecExt: Spec {

    type Key: Ord + Clone + ToString ;

    const LABEL: &'static str;

    // convert kubernetes objects into KV value
    fn convert_from_k8(k8_obj: K8Obj<Self,<Self as Spec>::Status>) -> 
           Result<MetaItem<Self,Self::Status,Self::Key>,IoError>;
}


/// Metadata object. Used to be KVObject int sc-core
#[derive(Debug, Clone, PartialEq)]
pub struct MetaItem<S,P,K>  {
    pub spec: S,
    pub status: P,
    pub key: K,
    pub ctx: MetaItemContext
}

impl <S>MetaItem<S,S::Status,S::Key> 
    where
        S: SpecExt 
{
    pub fn new<J>(key: J, spec: S, status: S::Status) -> Self where J: Into<S::Key> {
        Self {
            key: key.into(),
            spec,
            status,
            ctx: MetaItemContext::default()
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
    pub fn status(&self) -> &<S as Spec>::Status {
        &self.status
    }

    pub fn ctx(&self) -> &MetaItemContext {
        &self.ctx
    }

    pub fn set_ctx(&mut self, ctx: MetaItemContext) {
        self.ctx = ctx;
    }

    pub fn parts(self) -> (S::Key,S,MetaItemContext) {
        (self.key,self.spec,self.ctx)
    }

    pub fn is_owned(&self,uid: &str) -> bool {
        match &self.ctx.parent_ctx {
            Some(parent) => parent.uid == uid,
            None => false
        }
    }

}

impl <S>MetaItem<S,S::Status,S::Key> 
    where
        S: SpecExt,
        S::Status: Default
{

     pub fn with_spec<J>(key: J,spec: S) -> Self where J: Into<S::Key> {
        Self::new(key.into(),spec,S::Status::default())
    }

}



impl <S>fmt::Display for MetaItem<S,S::Status,S::Key> 
    where 
        S: SpecExt, 
        S::Key: Display
{

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"MetaItem {} key: {}",S::metadata().names.kind,self.key())
    }
}


impl <S>Into<(S::Key,S,S::Status)> for MetaItem<S,S::Status,S::Key> where S: SpecExt {
    fn into(self) -> (S::Key,S,S::Status) {
        (self.key,self.spec,self.status)
    }
}



#[derive(Debug, PartialEq, Clone)]
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

impl ::std::default::Default for MetaItemContext {
    fn default() -> Self {
        Self {
            item_ctx: None,
            parent_ctx: None,
        }
    }
}