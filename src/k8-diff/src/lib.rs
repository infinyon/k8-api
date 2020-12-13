mod json;

use std::fmt;

pub trait Changes {
    type Replace;
    type Patch;

    fn diff(&self, new: &Self) -> Result<Diff<Self::Replace, Self::Patch>, DiffError>;
}

#[derive(Debug)]
pub enum DiffError {
    DiffValue, // json values are different
}

impl std::fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON value types are different")
    }
}

impl std::error::Error for DiffError {}

// use Option as inspiration
#[derive(Debug)]
pub enum Diff<R, P> {
    None,
    Delete,
    Patch(P),   // for non primitive type
    Replace(R), // can be used for map and list (with our without tag), works on ordered list
    Merge(R),   // need tag, works on unorderd list
}

impl<R, P> Diff<R, P> {
    pub fn is_none(&self) -> bool {
        matches!(self, Diff::None)
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, Diff::Delete)
    }

    pub fn is_replace(&self) -> bool {
        matches!(self, Diff::Replace(_))
    }

    pub fn is_patch(&self) -> bool {
        matches!(self, Diff::Patch(_))
    }

    pub fn is_merge(&self) -> bool {
        matches!(self, Diff::Merge(_))
    }

    pub fn as_replace_ref(&self) -> &R {
        match self {
            Diff::Replace(ref val) => val,
            _ => panic!("no change value"),
        }
    }

    pub fn as_patch_ref(&self) -> &P {
        match self {
            Diff::Patch(ref val) => val,
            _ => panic!("no change value"),
        }
    }
}
