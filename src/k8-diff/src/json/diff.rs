use serde_json::Value;

use super::JsonDiff;
use super::PatchObject;
use crate::Changes;
use crate::Diff;
use crate::DiffError;

impl Changes for Value {
    type Replace = Value;
    type Patch = PatchObject;

    fn diff(&self, new: &Self) -> Result<JsonDiff, DiffError> {
        if *self == *new {
            return Ok(Diff::None);
        }
        match self {
            Value::Null => Ok(Diff::Replace(new.clone())),
            _ => {
                match new {
                    Value::Null => Ok(Diff::Replace(Value::Null)),
                    Value::Bool(ref _val) => Ok(Diff::Replace(new.clone())), // for now, we only support replace
                    Value::Number(ref _val) => Ok(Diff::Replace(new.clone())),
                    Value::String(ref _val) => Ok(Diff::Replace(new.clone())),
                    Value::Array(ref _val) => Ok(Diff::Replace(new.clone())),
                    Value::Object(ref new_val) => match self {
                        Value::Object(ref old_val) => {
                            let patch = PatchObject::diff(old_val, new_val)?;
                            Ok(Diff::Patch(patch))
                        }
                        _ => Err(DiffError::DiffValue),
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod test {

    use serde_json::json;
    use serde_json::Value;

    use super::Changes;

    #[test]
    fn test_null_comparision() {
        let n1 = Value::Null;
        let str1 = Value::String("test".to_owned());
        let str2 = Value::String("test".to_owned());

        assert!(n1.diff(&str1).expect("diff").is_replace());
        assert!(str1.diff(&str2).expect("diff").is_none());
    }

    #[test]
    fn test_object_comparision() {
        let old_spec = json!({
            "replicas": 2,
            "apple": 5
        });
        let new_spec = json!({
            "replicas": 3,
            "apple": 5
        });

        let diff = old_spec.diff(&new_spec).expect("diff");
        assert!(diff.is_patch());
        let patch = diff.as_patch_ref().get_inner_ref();
        assert_eq!(patch.len(), 1);
        let diff_replicas = patch.get("replicas").unwrap();
        assert!(diff_replicas.is_replace());
        assert_eq!(*diff_replicas.as_replace_ref(), 3);
    }

    #[test]
    #[allow(clippy::clippy::assertions_on_constants)]
    fn test_replace_some_with_none() {
        use serde::Serialize;
        use serde_json::to_value;

        use crate::Diff;

        #[derive(Serialize)]
        struct Test {
            choice: Option<bool>,
            value: u16,
        }

        let old_spec = to_value(Test {
            choice: Some(true),
            value: 5,
        })
        .expect("json");
        let new_spec = to_value(Test {
            choice: None,
            value: 5,
        })
        .expect("json");

        let diff = old_spec.diff(&new_spec).expect("diff");

        assert!(diff.is_patch());

        match diff {
            Diff::Patch(p) => {
                let json_diff = serde_json::to_value(p).expect("json");
                println!("json diff: {:#?}", json_diff);
                assert_eq!(json_diff, json!({ "choice": null }));
            }
            _ => assert!(false),
        }
    }
}
