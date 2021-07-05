use serde::Serialize;
use std::fmt::Debug;

pub trait RedisObject: Serialize + Debug {
    fn domain_prefix() -> String;

    fn prefix(&self) -> String {
        Self::domain_prefix()
    }

    fn plural_prefix(&self) -> String {
        format!("{}s", self.prefix())
    }

    fn hash(&self) -> String;

    fn domain(&self) -> String {
        return format!("{}:{}", Self::domain_prefix(), &self.hash());
    }

    fn json(&self) -> String {
        serde_json::to_string(&self).expect("I should be Serialize-able")
    }
}
