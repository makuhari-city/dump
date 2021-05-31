use crate::RedisObject;
use bs58::encode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Tag {
    pub uid: String,
    pub hash: String,
    pub title: String,
}

impl Tag {
    pub fn new(uid: &str, hash: &str, title: &str) -> Self {
        Self {
            uid: uid.to_string(),
            hash: hash.to_string(),
            title: title.to_string(),
        }
    }
}

impl RedisObject for Tag {
    fn domain_prefix() -> std::string::String {
        "tag".to_string()
    }

    fn id(&self) -> String {
        self.uid.to_string() // uid
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Voters(pub BTreeMap<String, BTreeMap<String, f64>>);

#[derive(Debug, Deserialize, Serialize)]
pub struct VoteParams {
    pub is_quadratic: Option<bool>,
    pub is_normalize: Option<bool>,
    pub voters: Voters,
}

impl RedisObject for VoteParams {
    fn domain_prefix() -> String {
        "params".to_string()
    }

    fn id(&self) -> String {
        let slice = serde_json::to_vec(&self).unwrap();
        encode(Sha256::digest(&slice)).into_string()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VoteInfo {
    pub uid: String,
    pub title: String,
    pub description: String,
    pub parent: Option<String>,
    pub method: String,
    pub params: VoteParams,
}

impl VoteInfo {
    pub fn dummy() -> Self {
        let voters: BTreeMap<String, BTreeMap<String, f64>> = serde_json::from_str(
            r#"
        { 
            "minori": {
                "yasushi": 0.1,
                "ray": 0.1,
                "rice": 0.1,
                "bread": 0.7
            },
            "yasushi": {
                "minori": 0.2,
                "ray": 0.3,
                "rice": 0.5
            },
            "ray": {
                "minori": 0.4,
                "yasushi": 0.4,
                "bread": 0.2
            }
        }"#,
        )
        .unwrap();

        let params = VoteParams {
            voters: Voters(voters),
            is_normalize: None,
            is_quadratic: None,
        };

        Self {
            uid: "56902678-14f7-4047-b7d4-da9efc3a5f7e".to_string(),
            title: "what to eat for breakfast".to_string(),
            description: "dummy VoteInfo".to_string(),
            method: "liquid".to_string(),
            parent: None,
            params,
        }
    }

    pub fn update_parent(&mut self, parent_id: String) {
        self.parent = Some(parent_id);
    }

    pub fn param_hash(&self) -> String {
        self.params.id()
    }
}

impl RedisObject for VoteInfo {
    fn domain_prefix() -> String {
        "info".to_string()
    }

    fn id(&self) -> String {
        // hash this
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.uid.as_bytes());
        bytes.extend_from_slice(&self.param_hash().as_bytes());
        bytes.extend_from_slice(&self.title.as_bytes());
        bytes.extend_from_slice(&self.description.as_bytes());
        bytes.extend_from_slice(&self.method.as_bytes());
        encode(Sha256::digest(&bytes)).into_string()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VoteResult {
    pub info_uid: String,
    pub info_hash: String,
    pub data: Value,
}

impl VoteResult {
    pub fn new(uid: &str, hash: &str, data: &Value) -> Self {
        Self {
            info_uid: uid.to_string(),
            info_hash: hash.to_string(),
            data: data.to_owned(),
        }
    }

    pub fn dummy() -> Self {
        let result: Value = serde_json::from_str(
            r#"
        [
            {
              "bread": 1.8275000000000001,
              "rice": 1.1724999999999999
            },
            {
              "minori": 1.9090909090909094,
              "ray": 1.4591836734693877,
              "yasushi": 1.6041666666666663
            }
        ]"#,
        )
        .unwrap();

        VoteResult {
            info_uid: "56902678-14f7-4047-b7d4-da9efc3a5f7e".to_string(),
            info_hash: "FLec22jSykQCWWAMi2svwxdte5VqKHj87CPz1mRZr8uX".to_string(),
            data: result,
        }
    }
}

impl RedisObject for VoteResult {
    fn domain_prefix() -> String {
        "result".to_string()
    }

    fn id(&self) -> String {
        self.info_hash.to_string()
    }
}
