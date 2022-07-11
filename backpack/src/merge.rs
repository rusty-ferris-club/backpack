use serde_json::Error;
use serde_json::Value;

fn to_value<T: serde::ser::Serialize>(value: &T) -> Result<serde_json::Value, Error> {
    serde_json::to_value(value)
}

fn from_value<T: serde::ser::Serialize + serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, Error> {
    serde_json::from_value(value)
}

fn merge_value(a: &mut Value, b: &Value) {
    match (a, b) {
        (Value::Object(ref mut a), &Value::Object(ref b)) => {
            for (k, v) in b {
                merge_value(a.entry(k).or_insert(Value::Null), v);
            }
        }
        (Value::Array(ref mut a), &Value::Array(ref b)) => {
            a.extend(b.clone());
        }
        (Value::Array(ref mut a), &Value::Object(ref b)) => {
            a.extend([Value::Object(b.clone())]);
        }
        (_, Value::Null) => {} // do nothing
        (a, b) => {
            *a = b.clone();
        }
    }
}

pub fn merge<T: serde::ser::Serialize + serde::de::DeserializeOwned>(
    base: &T,
    overrides: &T,
) -> Result<T, Error> {
    let mut left = to_value(base)?;
    let right = to_value(overrides)?;
    merge_value(&mut left, &right);
    from_value(left)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Serialize};

    use super::*;
    use insta::assert_yaml_snapshot;

    #[derive(Serialize, Deserialize)]
    struct Data {
        is_root: Option<bool>,
        folders: Vec<Folder>,
        entries: Option<BTreeMap<String, Entry>>, // btree so test results will be ordered and stable between runs
    }

    #[derive(Serialize, Deserialize)]
    struct Folder {
        name: String,
        num_files: Option<u32>,
    }

    #[derive(Serialize, Deserialize)]
    struct Entry {
        name: String,
        size: u32,
    }
    #[test]
    fn test_merge_left_empty() {
        let left: Data = serde_json::from_str(
            r###"
        {
            "is_root": false,
            "folders": []
        }
        "###,
        )
        .unwrap();
        let right: Data = serde_json::from_str(
            r###"
        {
            "is_root": true,
            "folders":[
                {
                    "name": "/var/log",
                    "num_files": 20
                }
            ],
            "entries": {
                "/var/log/f1": {
                    "name":"f1",
                    "size": 12
                }
            }
        }
        "###,
        )
        .unwrap();
        assert_yaml_snapshot!(merge(&left, &right).unwrap());
    }
    #[test]
    fn test_merge_right_empty() {
        let right: Data = serde_json::from_str(
            r###"
        {
            "is_root": false,
            "folders": []
        }
        "###,
        )
        .unwrap();
        let left: Data = serde_json::from_str(
            r###"
        {
            "is_root": true,
            "folders":[
                {
                    "name": "/var/log",
                    "num_files": 20
                }
            ],
            "entries": {
                "/var/log/f1": {
                    "name":"f1",
                    "size": 12
                }
            }
        }
        "###,
        )
        .unwrap();
        assert_yaml_snapshot!(merge(&left, &right).unwrap());
    }

    #[test]
    fn test_merge() {
        let left: Data = serde_json::from_str(
            r###"
        {
            "is_root": false,
            "entries": {
                "/var/log/f2": {
                    "name":"f2",
                    "size": 5
                }
            },
            "folders": [
                {
                    "name": "/var/log",
                    "num_files": 20
                }
            ]
        }
        "###,
        )
        .unwrap();
        let right: Data = serde_json::from_str(
            r###"
        {
            "folders":[],
            "entries": {
                "/var/log/f1": {
                    "name":"f1",
                    "size": 12
                }
            }
        }
        "###,
        )
        .unwrap();
        assert_yaml_snapshot!(merge(&left, &right).unwrap());
    }
}
