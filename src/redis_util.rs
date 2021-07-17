use crate::{model::RepresentativeInfo, RedisObject};
use actix::Addr;
use actix_redis::{Command, RedisActor};
use actix_web::{web, Error as AWError, HttpResponse};
use futures::future::{join, join_all};
use redis_async::{resp::RespValue as Value, resp_array};
use serde_json::json;
use uuid::Uuid;

// TODO this is obscuring the error, not best practice
pub async fn add(obj: &impl RedisObject, redis: &web::Data<Addr<RedisActor>>) -> Option<String> {
    let hash = obj.hash();
    let add = redis.send(Command(resp_array!["SET", obj.domain(), obj.json()]));
    let list = redis.send(Command(resp_array!["SADD", &obj.plural_prefix(), &hash]));
    let (add, _list) = join(add, list).await;
    if let Ok(Ok(Value::SimpleString(x))) = add {
        if x == "OK" {
            Some(hash)
        } else {
            None
        }
    } else {
        None
    }
}

pub async fn push_representative(
    id: &str,
    info: &RepresentativeInfo,
    redis: &web::Data<Addr<RedisActor>>,
) -> Option<i64> {
    let json_data = serde_json::to_string(&info).expect("info should be serializable");

    let push = redis
        .send(Command(resp_array![
            "HSET",
            "reps",
            id.to_string(),
            json_data
        ]))
        .await;
    if let Ok(Ok(Value::Integer(x))) = push {
        Some(x)
    } else {
        None
    }
}

pub async fn get_representatives(redis: &web::Data<Addr<RedisActor>>) -> Option<Vec<String>> {
    let get_reps = redis.send(Command(resp_array!["HKEYS", "reps"])).await;

    if let Ok(Ok(Value::Array(r))) = get_reps {
        let reps: Vec<String> = r
            .iter()
            .map(|v| match v {
                Value::BulkString(x) => String::from_utf8(x.to_owned()).unwrap().to_owned(),
                _ => "".to_string(),
            })
            .filter(|string| string != &("".to_string()))
            .collect();
        Some(reps)
    } else {
        None
    }
}

pub async fn get_representative(
    id: &str,
    redis: &web::Data<Addr<RedisActor>>,
) -> Option<RepresentativeInfo> {
    let get_rep = redis.send(Command(resp_array!["HGET", "reps", id])).await;

    if let Ok(Ok(Value::BulkString(r))) = get_rep {
        let rep: RepresentativeInfo =
            serde_json::from_slice(&r).expect("Rep Info should be serializable");
        Some(rep)
    } else {
        None
    }
}

pub async fn push_history(
    id: &Uuid,
    hash: &str,
    redis: &web::Data<Addr<RedisActor>>,
) -> Option<i64> {
    let domain = format!("history:{}", id);
    let push = redis
        .send(Command(resp_array!["RPUSH", &domain, hash]))
        .await;

    if let Ok(Ok(Value::Integer(x))) = push {
        Some(x)
    } else {
        None
    }
}

pub async fn get_history(id: &str, redis: &web::Data<Addr<RedisActor>>) -> Option<Vec<String>> {
    let domain = format!("history:{}", id);
    let history = redis
        .send(Command(resp_array!["LRANGE", &domain, "0", "-1"]))
        .await;

    if let Ok(Ok(Value::Array(hs))) = history {
        let hashes: Vec<String> = hs
            .iter()
            .map(|v| match v {
                Value::BulkString(x) => String::from_utf8(x.to_owned()).unwrap().to_owned(),
                _ => "".to_string(),
            })
            .filter(|string| string != &("".to_string()))
            .rev()
            .collect();
        Some(hashes)
    } else {
        None
    }
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
