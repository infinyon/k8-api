use std::{str::FromStr, convert::Infallible};

/// See: https://github.com/kubernetes/apimachinery/blob/master/pkg/util/intstr/intstr.go
/// Int32OrString is a type that can hold an int32 or a string.
/// When used in JSON or YAML marshalling and unmarshalling, it produces or consumes the inner type.
/// This allows you to have, for example, a JSON field that can accept a name or number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Int32OrString {
    Int(i32),
    String(String),
}

impl Default for Int32OrString {
    fn default() -> Self {
        Int32OrString::Int(0)
    }
}
impl FromStr for Int32OrString {
    type Err = Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<i32>() {
            Ok(i) => Ok(Int32OrString::Int(i)),
            Err(_) => Ok(Int32OrString::String(s.to_string())),
        }
    }
}

impl From<i32> for Int32OrString {
    fn from(f: i32) -> Self {
        Int32OrString::Int(f)
    }
}

impl<'de> serde::Deserialize<'de> for Int32OrString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = Int32OrString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "enum Int32OrString")
            }

            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Int32OrString::Int(v))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v < i64::from(i32::MIN) || v > i64::from(i32::MAX) {
                    return Err(serde::de::Error::invalid_value(
                        serde::de::Unexpected::Signed(v),
                        &"a 32-bit integer",
                    ));
                }

                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Ok(Int32OrString::Int(v as i32))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                #[allow(clippy::cast_sign_loss)]
                {
                    if v > i32::MAX as u64 {
                        return Err(serde::de::Error::invalid_value(
                            serde::de::Unexpected::Unsigned(v),
                            &"a 32-bit integer",
                        ));
                    }
                }

                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Ok(Int32OrString::Int(v as i32))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_string(v.to_string())
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(Int32OrString::String(v))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl serde::Serialize for Int32OrString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Int32OrString::Int(i) => i.serialize(serializer),
            Int32OrString::String(s) => s.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod test {

    use serde_json::json;

    use crate::Int32OrString;

    #[test]
    fn test_int_serde() {
        let int_value = json!(100);

        let int_or_string: Int32OrString =
            serde_json::from_value(int_value.clone()).expect("failed deserialization");
        assert_eq!(int_or_string, Int32OrString::Int(100));
        let serialization = serde_json::to_value(&int_or_string).expect("failed serialization");
        assert_eq!(int_value, serialization);
    }

    #[test]
    fn test_invalid_float_serde() {
        let int_value = json!(2.5);

        let _error = serde_json::from_value::<Int32OrString>(int_value)
            .expect_err("float should not be deserialized");
    }

    #[test]
    fn test_str_serde() {
        let str_value = json!("25%");

        let int_or_string: Int32OrString =
            serde_json::from_value(str_value.clone()).expect("failed deserialization");
        assert_eq!(int_or_string, Int32OrString::String("25%".into()));

        let serialization = serde_json::to_value(&int_or_string).expect("failed serialization");
        assert_eq!(str_value, serialization);
    }
}
