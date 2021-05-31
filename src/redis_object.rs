use serde::Serialize;
use std::fmt::Debug;

pub trait RedisObject: Serialize + Debug {
    fn domain_prefix() -> String;

    fn prefix(&self) -> String {
        Self::domain_prefix()
    }

    fn id(&self) -> String;
    fn domain(&self) -> String {
        return format!("{}:{}", Self::domain_prefix(), &self.id());
    }
    fn json(&self) -> String {
        serde_json::to_string(&self).expect("I should be Serialize-able")
    }
}
