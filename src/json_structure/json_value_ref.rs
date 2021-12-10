use super::Key;
use serde::Serialize;
use serde_json::{Map as JMap, Value as JsonValue};
use std::collections::HashMap;

pub enum JsonValueRef<'v, 'k: 'v> {
    Object(Vec<(Key<'k>, JsonValueRef<'v, 'k>)>),
    Array(Vec<&'v JsonValue>),
    Value(&'v JsonValue),
}

impl JsonValueRef<'_, '_> {
    pub fn into_json_value(self) -> JsonValue {
        match self {
            JsonValueRef::Object(key_values) => {
                let mut obj = JMap::with_capacity(key_values.len());
                for (k, v) in key_values.into_iter() {
                    obj.insert(k.to_string(), v.into_json_value());
                }
                JsonValue::Object(obj)
            }
            JsonValueRef::Array(arr) => {
                let arr_values: Vec<JsonValue> = arr.into_iter().map(|each| each.clone()).collect();
                JsonValue::Array(arr_values)
            }
            JsonValueRef::Value(v) => v.clone(),
        }
    }
}

impl<'v, 'k> Serialize for JsonValueRef<'v, 'k> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: ::serde::Serializer,
    {
        match *self {
            JsonValueRef::Value(ref v) => v.serialize(serializer),
            JsonValueRef::Array(ref vs) => vs.serialize(serializer),
            JsonValueRef::Object(ref vec) => {
                let mut obj: HashMap<&str, &JsonValueRef> = HashMap::with_capacity(vec.len());
                for (k, v) in vec {
                    obj.insert(k, v);
                }
                obj.serialize(serializer)
            }
        }
    }
}
