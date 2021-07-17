use crate::RedisObject;
use bs58::encode;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use vote::{TopicData, VoteData};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepresentativeInfo {
    name: String,
    link: Option<String>,
    info: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicHeader {
    pub id: Uuid,
    pub hash: String,
    pub title: String,
}

impl TopicHeader {
    pub fn new(id: &Uuid, hash: &str, title: &str) -> Self {
        Self {
            id: id.to_owned(),
            hash: hash.to_string(),
            title: title.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TopicCalculationResult {
    pub topic_hash: String,
    data: serde_json::Value,
}

impl TopicCalculationResult {
    pub fn new(topic_hash: &str, data: &serde_json::Value) -> Self {
        Self {
            topic_hash: topic_hash.to_string(),
            data: data.to_owned(),
        }
    }
}

impl RedisObject for TopicCalculationResult {
    fn domain_prefix() -> String {
        "result".to_string()
    }

    fn hash(&self) -> String {
        self.topic_hash.to_string()
    }
}

impl RedisObject for TopicHeader {
    fn domain_prefix() -> String {
        "header".to_string()
    }

    fn hash(&self) -> String {
        self.id.to_string()
    }
}

impl RedisObject for TopicData {
    fn domain_prefix() -> String {
        "topic".to_string()
    }

    fn hash(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(self.title.as_bytes());
        hash.update(self.description.as_bytes());

        // FIXME: This is ugly. we are hasing the key and value of policies and delegates
        // separately...
        let p_values = self
            .policies_values()
            .iter()
            .fold("".to_string(), |acc, d| format!("{}{}", acc, d));
        hash.update(p_values.as_bytes());
        let d_values = self
            .delegates_values()
            .iter()
            .fold("".to_string(), |acc, d| format!("{}{}", acc, d));
        hash.update(d_values.as_bytes());
        let vote: VoteData = self.to_owned().into();
        hash.update(&serde_json::to_vec(&vote).expect("VoteData should be Serializeable."));

        encode(hash.finalize()).into_string()
    }
}
