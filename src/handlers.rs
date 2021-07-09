use crate::{
    ipfs::post_ipfs,
    model::{TopicCalculationResult, TopicHeader},
    redis_util, RedisObject,
};
use actix::Addr;
use actix_redis::RedisActor;
use actix_web::{get, post, web, Responder};
use futures::{future::join_all, join};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
use vote::{TopicData, VoteData};

#[get("/hello/")]
pub async fn hello() -> impl Responder {
    "hello".to_string()
}

#[get("/list/")]
pub async fn get_list(redis: web::Data<Addr<RedisActor>>) -> impl Responder {
    let ids = redis_util::get_list("header", &redis).await;

    if ids.is_none() {
        return web::Json(json!({"status":"error", "mes":"could not get tag list"}));
    };

    log::info!("{:?}", &ids);

    let tags: Vec<TopicHeader> = join_all(
        ids.unwrap()
            .iter()
            .map(|tag| redis_util::get_slice(tag, "header", &redis)),
    )
    .await
    .iter()
    .filter(|result| result.is_some())
    .map(|tag| serde_json::from_slice(&(tag.to_owned().unwrap())).unwrap())
    .collect();

    web::Json(serde_json::to_value(tags).unwrap())
}

#[get("/header/{id}")]
pub async fn get_header(
    id: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    match redis_util::get_slice(&id, "header", &redis)
        .await
        .and_then(|slice| serde_json::from_slice::<TopicHeader>(&slice).ok())
    {
        Some(th) => web::Json(json!(th)),
        None => web::Json(json!({"status":"errro", "mes": "error fetching header"})),
    }
}

#[get("/history/{id}/")]
pub async fn history(id: web::Path<String>, redis: web::Data<Addr<RedisActor>>) -> impl Responder {
    let id = id.into_inner();
    let history = redis_util::get_history(&id, &redis).await;

    match history {
        Some(h) => web::Json(serde_json::to_value(h).unwrap()),
        None => web::Json(json!({"status":"error", "mes":"history not found"})),
    }
}

#[post("/topic/raw/")]
pub async fn post_topic_raw(
    topic: web::Json<TopicData>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let topic: TopicData = topic.into_inner();

    // check if uuid is alreay in tags
    let header: Option<TopicHeader> =
        redis_util::get_slice(&topic.id.to_string(), "header", &redis)
            .await
            .and_then(|v| serde_json::from_slice(&v).unwrap());

    let topic_hash = topic.hash();

    let new_header = match header {
        Some(t) if t.hash == topic_hash => {
            // check if tag's hash is the same as the current topic
            // if it's the same do nothing. data won't change
            return web::Json(
                json!({"status":"ok", "hash":t.hash, "id": t.id, "mes":"dup found, no change"}),
            );
        }
        _ => TopicHeader::new(&topic.id, &topic_hash, &topic.title),
    };

    let write_hash = post_ipfs(&topic_hash);
    let push_history = redis_util::push_history(&topic.id, &topic_hash, &redis);
    let update_tag = redis_util::add(&new_header, &redis);
    let add_topic = redis_util::add(&topic, &redis);
    let (id, _hash, _history, _ipfs) = join!(update_tag, add_topic, push_history, write_hash);
    match id {
        Some(id) => return web::Json(json!({"status":"ok", "hash": topic_hash, "id": id})),
        None => {
            return web::Json(
                json!({"status":"error", "mes":"could not update tag and post topic."}),
            )
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PartialTopic {
    title: String,
    description: String,
}

#[post("/topic/new/")]
pub async fn make_new_topic(
    partial: web::Json<PartialTopic>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let partial = partial.into_inner();

    let new_topic = TopicData::new(&partial.title, &partial.description);
    let new_header = TopicHeader::new(&new_topic.id, &new_topic.hash(), &partial.title);

    let (hash, _head) = join!(
        redis_util::add(&new_topic, &redis),
        redis_util::add(&new_header, &redis)
    );

    match hash {
        Some(h) => web::Json(json!({"status":"ok", "id":new_topic.id, "hash":h})),
        None => web::Json(json!({"status":"error", "mes": "error adding new topic"})),
    }
}

#[post("/topic/update/{id}/{field}/")]
pub async fn update_field(
    path: web::Path<(String, String)>,
    new_info: web::Json<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let (id, field) = path.into_inner();

    let new = new_info.into_inner();

    let data = get_latest_id(&id, &redis).await;

    if data.is_none() {
        return web::Json(json!({"status":"error", "mes": "could not find table data under id."}));
    }

    let mut data = data.unwrap();

    match field.as_ref() {
        "title" => data.title = new,
        "description" => data.description = new,
        _ => return web::Json(json!({"status":"error", "mes": "invalid field"})),
    };

    match update_topic_data(&data, &redis).await {
        true => web::Json(json!({"status":"ok"})),
        _ => web::Json(json!({"status":"error", "mes":"failed to update topic data."})),
    }
}

#[post("/topic/update/{id}/policy/")]
pub async fn add_policy(
    id: web::Path<String>,
    policy: web::Json<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let id = id.into_inner();
    let policy = policy.into_inner();

    let data = get_latest_id(&id, &redis).await;

    if data.is_none() {
        return web::Json(json!({"status":"error", "mes":"failed to get topic data"}));
    }

    let mut data = data.unwrap();

    let _id = data.add_new_policy(&policy);

    let vote: VoteData = data.to_owned().into();

    println!("vote:{:?}", vote);

    match update_topic_data(&data, &redis).await {
        true => web::Json(json!(&data)),
        _ => web::Json(json!({"status":"error", "mes": "failed to update data"})),
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserVote {
    id: Uuid,
    name: String,
    vote: BTreeMap<Uuid, f64>,
}

#[post("/topic/update/{id}/delegate/")]
pub async fn update_vote(
    path: web::Path<String>,
    uservote: web::Json<UserVote>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let id = path.into_inner();
    let uservote = uservote.into_inner();

    let data = get_latest_id(&id, &redis).await;

    if data.is_none() {
        return web::Json(json!({ "status":"error", "mes":"could not find topic data." }));
    };

    let mut data = data.unwrap();

    // is this uid already registered?
    if !data.votes().iter().any(|(p, _)| p == &uservote.id) {
        let _ = data.add_delegate(&uservote.id, &uservote.name);
    };

    data.overwrite_vote_for(uservote.id, uservote.vote);

    match update_topic_data(&data, &redis).await {
        true => web::Json(json!(&data)),
        _ => web::Json(json!({"status":"error", "mes": "failed to update data"})),
    }
}

pub async fn get_latest_id(id: &str, redis: &web::Data<Addr<RedisActor>>) -> Option<TopicData> {
    let hash = redis_util::get_slice(id, "header", &redis)
        .await
        .and_then(|slice| serde_json::from_slice::<TopicHeader>(&slice).ok())
        .and_then(|header| Some(header.hash));

    if hash.is_none() {
        return None;
    }

    let hash = hash.unwrap();

    redis_util::get_slice(&hash, "topic", &redis)
        .await
        .and_then(|slice| serde_json::from_slice::<TopicData>(&slice).ok())
}

pub async fn update_topic_data(data: &TopicData, redis: &web::Data<Addr<RedisActor>>) -> bool {
    let header: Option<TopicHeader> = redis_util::get_slice(&data.id.to_string(), "header", &redis)
        .await
        .and_then(|slice| serde_json::from_slice(&slice).ok());

    log::info!("header:{:?}", header);

    if header.is_none() {
        // no header, we don't update
        return false;
    }

    let data_hash = data.hash();
    let header = header.unwrap();

    log::info!("{}={}", &header.hash, &data_hash);

    if &header.hash == &data_hash {
        // same data, we don't update
        return false;
    }

    let push_history = redis_util::push_history(&data.id, &data_hash, &redis);
    let new_header = TopicHeader::new(&data.id, &data.hash(), &data.title);
    let update_header = redis_util::add(&new_header, &redis);
    let post_data = redis_util::add(data, &redis);

    let (_, _, hash) = join!(push_history, update_header, post_data);

    log::info!("hash: {:?}", &hash);

    match hash {
        Some(_h) => true,
        _ => false,
    }
}

#[get("/topic/raw/{hash}/")]
pub async fn get_topic_raw(
    hash: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let hash = hash.into_inner();
    match redis_util::get_slice(&hash, "topic", &redis).await {
        Some(s) => {
            let topic: TopicData =
                serde_json::from_slice(&s).expect("topic should be Deserializeable");
            web::Json(serde_json::to_value(topic).expect("this also should be Serializable"))
        }
        None => web::Json(json!({"status":  "error", "message": "could not find topic"})),
    }
}

#[get("/topic/{id}/")]
pub async fn get_topic_by_id(
    id: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let id = id.into_inner();

    let latest_hash = redis_util::get_slice(&id, "header", &redis)
        .await
        .and_then(|header| {
            Some(
                serde_json::from_slice::<TopicHeader>(&header)
                    .expect("Topic Header is Serializable"),
            )
        })
        .and_then(|h| Some(h.hash));

    match latest_hash {
        Some(s) => {
            let topic = redis_util::get_slice(&s, "topic", &redis)
                .await
                .and_then(|b| Some(serde_json::from_slice::<TopicData>(&b).unwrap()));
            match topic {
                Some(t) => web::Json(json!(t)),
                None => web::Json(json!({"status":  "error", "message": "could not find topic"})),
            }
        }
        None => web::Json(json!({"status":  "error", "message": "could not find topic"})),
    }
}

#[post("/result/{hash}/")]
pub async fn dump_result(
    hash: web::Path<String>,
    data: web::Json<serde_json::Value>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let hash = hash.into_inner();
    let data = data.into_inner();
    let result = TopicCalculationResult::new(&hash, &data);

    match redis_util::add(&result, &redis).await {
        Some(hash) => web::Json(json!({"status":"ok", "hash": hash})),
        None => web::Json(json!({"status":"error", "mes": "could not add result"})),
    }
}

#[get("/result/{hash}/")]
pub async fn get_result(
    hash: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> impl Responder {
    let hash = hash.into_inner();

    match redis_util::get_slice(&hash, "result", &redis)
        .await
        .and_then(|bytes| serde_json::from_slice::<TopicCalculationResult>(&bytes).ok())
    {
        Some(result) => web::Json(json!(result)),
        None => web::Json(json!({"status":"error", "mes":"could not find result"})),
    }
}
