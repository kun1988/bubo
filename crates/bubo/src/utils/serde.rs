use std::collections::HashSet;

use serde::{de, Deserialize, Deserializer};
use serde_json::Value;


// 可以通过该函数兼容不同类型或者直接报错
pub fn to_i64<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'de>,
{           
    let v: Value = Deserialize::deserialize(deserializer)?;

    if v.is_string() {
        let r = v.as_str().unwrap();
        let r = r.parse::<i64>().map_err(|_e| de::Error::custom("string parse int error"))?;
        Ok(r)
    } else if v.is_i64() {
        let r = v.as_i64().unwrap();
        Ok(r)
    } else {
        Err(de::Error::custom("type error"))
    }
}

pub fn to_vec_i64<'de, D>(deserializer: D) -> Result<Vec<i64>, D::Error>
where
    D: Deserializer<'de>,
{           
    let v: Vec<Value> = Deserialize::deserialize(deserializer)?;
    let r: Result<Vec<i64>, D::Error> = v.into_iter().map(|s| {
        if s.is_string() {
            let r = s.as_str().unwrap();
            let r = r.parse::<i64>().map_err(|_e| de::Error::custom("string parse int error"));
            r
        } else if s.is_i64() {
            let r = s.as_i64().unwrap();
            Ok(r)
        } else {
            return Err(de::Error::custom("type error"));
        }
    }).collect();
    Ok(r?)
}

pub fn to_set_i64<'de, D>(deserializer: D) -> Result<HashSet<i64>, D::Error>
where
    D: Deserializer<'de>,
{           
    let v: Vec<Value> = Deserialize::deserialize(deserializer)?;
    let r: Result<HashSet<i64>, D::Error> = v.into_iter().map(|s| {
        if s.is_string() {
            let r = s.as_str().unwrap();
            let r = r.parse::<i64>().map_err(|_e| de::Error::custom("string parse int error"));
            r
        } else if s.is_i64() {
            let r = s.as_i64().unwrap();
            Ok(r)
        } else {
            return Err(de::Error::custom("type error"));
        }
    }).collect();
    Ok(r?)
}

pub fn to_i64_option<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{           
    let v = Option::<Value>::deserialize(deserializer)?;
    if v.is_none() {
        return Ok(None);
    }
    let v = v.unwrap();
    if v.is_string() {
        let r = v.as_str().unwrap();
        let r = r.parse::<i64>().map_err(|_e| de::Error::custom("string parse int error"))?;
        Ok(Some(r))
    } else if v.is_i64() {
        let r = v.as_i64().unwrap();
        Ok(Some(r))
    } else {
        Err(de::Error::custom("type error"))
    }
}