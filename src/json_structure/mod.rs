mod json_value_ref;

use json_value_ref::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

type Key<'a> = &'a str;

type Result<T> = std::result::Result<T, JsonStructureError>;

#[derive(Error, Debug, PartialEq)]
pub enum JsonStructureError {
    #[error("invalid json structure definition :{0}")]
    InvalidJsonStructureDef(String),

    #[error("invalid key :{0}")]
    InvalidKey(String),

    #[error("value out of range. key:{0}, idx:{1}")]
    ValueOutOfRange(String, usize),

    #[error("invalid structure state {0}")]
    InvalidStructureState(String),
}

///
/// ["col1","col2","col3"]
/// =>
/// ```ignore
///  //pseude code
/// Structure::Object{values:vec[("col1",Value(0)), ("col2",Value(1)), ("col3",Value(2))]}
/// ```
///
/// ["col1","col2","col3","col2"]
/// =>
///```ignore
///  //pseude code
///  Structure::Object{values:vec[("col1",Value(0)), ("col2",Array(vec![0,3])), ("col3",Value(2))]}
/// ```
///
/// ["col1","col2.greeding1","col3","col2.greeding2"]
///
/// =>
///```ignore
///  //pseude code
///  Structure::Object{
///    values:vec[("col1",Value(0)),
///               ("col2",Object(Object{values:vec![
///                 ("greeding1":Value(1)),
///                 ("greeding2":Value(3))
///               ]})),
///               ("col3",Value(2)) ]
///  }
///
///```
///
/// ["col1","col2.greeding1","col3","col2.greeding2.title"]
///
/// =>
///```ignore
///  //pseude code
///  Structure::Object{
///    values:vec[("col1",Value(0)),
///               ("col2",Object(Object{values:vec![
///                 ("greeding1":Value(1)),
///                 ("greeding2":Object(
///                     Object{values:vec![("title",Value(3))]}
///                 ))
///               ]})),
///               ("col3",Value(2)) ]
///  }
///
///```
///
#[derive(Debug, PartialEq)]
pub enum Structure<'a> {
    Object(Object<'a>),
    Array(Key<'a>, Vec<usize>),
    Value(Key<'a>, usize),
}

impl<'a> Structure<'a> {
    pub fn new_obj(obj: Object<'a>) -> Self {
        Self::Object(obj)
    }

    pub fn new_arr(k: Key<'a>, v: Vec<usize>) -> Self {
        Self::Array(k, v)
    }

    pub fn new_value(k: Key<'a>, v: usize) -> Self {
        Self::Value(k, v)
    }

    pub fn build_json<'v>(&'a self, values: &'v [&JsonValue]) -> Result<JsonValueRef<'v, 'a>> {
        match self {
            Structure::Object(obj) => obj.build_json(values),
            Structure::Array(key, indices) => {
                let mut result = Vec::<&JsonValue>::new();

                for index in indices {
                    match values.get(*index) {
                        None => {
                            return Err(JsonStructureError::ValueOutOfRange(
                                key.to_string(),
                                *index,
                            ))
                        }
                        Some(value) => result.push(value),
                    }
                }

                Ok(JsonValueRef::Array(result))
            }
            Structure::Value(key, index) => match values.get(*index) {
                None => return Err(JsonStructureError::ValueOutOfRange(key.to_string(), *index)),
                Some(value) => Ok(JsonValueRef::Value(value)),
            },
        }
    }
}

pub(crate) fn split_keys<'a>(keys: &'a str) -> Vec<Key<'a>> {
    keys.split(".").map(|e| e.trim()).collect()
}

fn key_seq_to_key_str<'a>(keys: &[Key<'a>]) -> String {
    keys.join(".")
}

fn key_seq_to_sub_key_str<'a>(keys: &[Key<'a>], end_idx: usize) -> String {
    keys[0..end_idx].join(".")
}

#[derive(Debug, PartialEq)]
pub struct Object<'a> {
    pub keys: Vec<Key<'a>>,
    pub values: HashMap<Key<'a>, Structure<'a>>,
}

impl<'a> Object<'a> {
    pub fn new() -> Self {
        Self {
            keys: Vec::new(),
            values: HashMap::new(),
        }
    }

    pub fn from_strs(strs: &'a [&str]) -> Result<Object<'a>> {
        let mut obj = Self::new();
        for (idx, each) in strs.iter().enumerate() {
            obj.add_value(each, idx)?;
        }
        Ok(obj)
    }

    pub fn contains_key(&self, key: Key<'a>) -> bool {
        self.values.contains_key(key)
    }

    pub fn get_mut(&mut self, key: Key<'a>) -> Option<&mut Structure<'a>> {
        self.values.get_mut(key)
    }

    fn add_value(&mut self, key: &'a str, idx: usize) -> Result<()> {
        let key_seq = split_keys(key);

        if key_seq.is_empty() {
            return Err(JsonStructureError::InvalidKey(format!(
                "invalid key:{}",
                key
            )));
        }
        if let Some(_) = key_seq.iter().find(|e| e.is_empty()) {
            return Err(JsonStructureError::InvalidKey(format!(
                "invalid key:{}",
                key
            )));
        }

        self.add_value_with_key_seq(0, key_seq.as_slice(), idx)
    }

    /// about key_seq: key_strings "obj1.key1.key2" turns into vec!["obj1","key1","key2"]
    ///
    pub fn add_value_with_key_seq(
        &mut self,
        key_idx: usize,
        key_seq: &[Key<'a>],
        idx: usize,
    ) -> Result<()> {
        if key_seq.len() <= key_idx {
            panic!(
                "keys seq is out of range.this must be a bug  key_seq.len() = {}, key_idx={}  ",
                key_seq.len(),
                key_idx
            )
        }

        // key_seq never be empty here
        let current_key = unsafe { key_seq.get_unchecked(key_idx) };
        let is_last_key_of_seq = (key_seq.len() - 1) == key_idx;

        if is_last_key_of_seq {
            let overwrite = match self.get_mut(current_key) {
                None => {
                    self.inner_add(*current_key, Structure::new_value(*current_key, idx));
                    None
                }
                Some(existing) => match existing {
                    Structure::Object(_) => {
                        return Err(JsonStructureError::InvalidJsonStructureDef(format!(
                            "key:`{}` is a value but also a object",
                            key_seq_to_key_str(key_seq)
                        )))
                    }
                    Structure::Array(_, arr) => {
                        arr.push(idx);
                        None
                    }
                    Structure::Value(key, existing_idx) => {
                        let new_arr = Structure::new_arr(key, vec![*existing_idx, idx]);
                        Some(new_arr)
                    }
                },
            };
            if let Some(overwrite) = overwrite {
                self.inner_add(current_key, overwrite)
            }
        } else {
            match self.get_mut(current_key) {
                None => {
                    let mut new_obj = Object::new();
                    new_obj.add_value_with_key_seq(key_idx + 1, key_seq, idx)?;
                    self.inner_add(current_key, Structure::Object(new_obj))
                }
                Some(existing) => match existing {
                    Structure::Object(obj) => {
                        obj.add_value_with_key_seq(key_idx + 1, key_seq, idx)?
                    }
                    Structure::Array(_, _arr) => {
                        return Err(JsonStructureError::InvalidJsonStructureDef(format!(
                            "key:`{}` supposed to be a object but array",
                            key_seq_to_sub_key_str(key_seq, key_idx)
                        )))
                    }
                    Structure::Value(_, _idx) => {
                        return Err(JsonStructureError::InvalidJsonStructureDef(format!(
                            "key:`{}` supposed to be a object but value",
                            key_seq_to_sub_key_str(key_seq, key_idx)
                        )))
                    }
                },
            }
        }
        Ok(())
    }

    fn inner_add(&mut self, key: Key<'a>, v: Structure<'a>) {
        if self.contains_key(key) {
            self.values.insert(key, v); //override
        } else {
            self.values.insert(key, v);
            self.keys.push(key);
        }
    }

    pub fn build_json<'v>(&'a self, values: &'v [&JsonValue]) -> Result<JsonValueRef<'v, 'a>> {
        let mut value_map = Vec::with_capacity(self.values.len());
        for each_key in &self.keys {
            match self.values.get(each_key) {
                None => {
                    return Err(JsonStructureError::InvalidStructureState(format!(
                        "key {} is not exists",
                        each_key
                    )))
                }
                Some(index) => {
                    let json_value = index.build_json(values)?;
                    value_map.push((*each_key, json_value));
                }
            }
        }

        Ok(JsonValueRef::Object(value_map))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use serde_json::json;

    use serde_json::Value as JsonValue;

    #[test]
    fn test_structure_1() {
        let mut obj = Object::new();
        let add_result = obj.add_value("hello", 0);
        assert!(add_result.is_ok());

        let mut expected_value = HashMap::new();
        expected_value.insert("hello", Structure::new_value("hello", 0));
        let expected = Object {
            keys: vec!["hello"],
            values: expected_value,
        };

        assert_eq!(obj, expected);
    }

    #[test]
    fn test_structure_2() {
        let mut obj = Object::new();
        let add_result = obj.add_value("hello", 0);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello2", 1);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello", 2);
        assert!(add_result.is_ok());

        let mut expected_value = HashMap::new();
        expected_value.insert("hello", Structure::new_arr("hello", vec![0, 2]));
        expected_value.insert("hello2", Structure::new_value("hello2", 1));
        let expected = Object {
            keys: vec!["hello", "hello2"],
            values: expected_value,
        };

        assert_eq!(obj, expected);
    }

    #[test]
    fn test_structure_3() {
        let mut obj = Object::new();
        let add_result = obj.add_value("hello.some", 0);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello.some2", 1);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello.some", 2);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hi", 3);
        assert!(add_result.is_ok());

        let mut expected_map = HashMap::new();
        expected_map.insert("some", Structure::new_arr("some", vec![0, 2]));
        expected_map.insert("some2", Structure::new_value("some2", 1));
        let expected_obj = Object {
            keys: vec!["some", "some2"],
            values: expected_map,
        };

        let mut expected_value = HashMap::new();
        expected_value.insert("hello", Structure::new_obj(expected_obj));
        expected_value.insert("hi", Structure::new_value("hi", 3));
        let expected = Object {
            keys: vec!["hello", "hi"],
            values: expected_value,
        };

        assert_eq!(obj, expected);
    }

    #[test]
    fn test_build_json_1() {
        let mut obj = Object::new();
        let add_result = obj.add_value("hello.some", 0);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello.some2", 1);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hello.some", 2);
        assert!(add_result.is_ok());

        let add_result = obj.add_value("hi", 3);
        assert!(add_result.is_ok());

        let obj = Structure::new_obj(obj);

        let v1 = "col1".into();
        let v2 = "col2".into();
        let v3 = "col3".into();
        let v4 = "col4".into();

        let values = vec![&v1, &v2, &v3, &v4];
        let build_result = obj.build_json(&values);
        assert!(build_result.is_ok());
        let build_result = build_result.unwrap();

        let build_result = serde_json::to_string(&build_result).unwrap();
        let build_result: JsonValue = serde_json::from_str(&build_result).unwrap();
        let expected = json!({
            "hello":{
                "some":["col1","col3"],
                "some2":"col2",
            },
            "hi":"col4",
        });

        assert_eq!(build_result, expected);
    }
}
