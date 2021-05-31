use crate::RedisObject;
use actix::Addr;
use actix_redis::{Command, RedisActor};
use actix_web::{web, Error as AWError, HttpResponse};
use futures::future::{join, join_all};
use redis_async::{resp::RespValue as Value, resp_array};
use serde_json::json;

// TODO this is obscuring the error, not best practice
pub async fn add(obj: &impl RedisObject, redis: &web::Data<Addr<RedisActor>>) -> bool {
    let add = redis.send(Command(resp_array!["SET", obj.domain(), obj.json()]));
    let plural_domain = format!("{}s", obj.prefix());
    let list = redis.send(Command(resp_array!["SADD", &plural_domain, &obj.id()]));
    let (add, _list) = join(add, list).await;
    if let Ok(Ok(Value::SimpleString(x))) = add {
        return x == "OK";
    }
    false
}

pub async fn get_slice(
    id: &str,
    domain_prefix: &str,
    redis: &web::Data<Addr<RedisActor>>,
) -> Option<Vec<u8>> {
    let domain = format!("{}:{}", domain_prefix, id);

    let obj = redis.send(Command(resp_array!["GET", &domain])).await;

    if let Ok(Ok(Value::BulkString(x))) = obj {
        Some(x)
    } else {
        None
    }
}

pub async fn get_list(domain: &str, redis: &web::Data<Addr<RedisActor>>) -> Option<Vec<String>> {
    let plural = format!("{}s", domain);
    let set = redis.send(Command(resp_array!["SMEMBERS", &plural])).await;
    match set {
        Ok(Ok(Value::Array(x))) => {
            let mut result = Vec::new();
            for e in x {
                if let Value::BulkString(x) = e {
                    let id = String::from_utf8(x).expect("id should be utf-8");
                    result.push(id);
                }
            }
            Some(result)
        }
        _ => return None,
    }
}
