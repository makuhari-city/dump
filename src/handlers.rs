use crate::{
    model::{Tag, VoteResult},
    redis_util, RedisObject, VoteInfo,
};
use actix::Addr;
use actix_redis::RedisActor;
use actix_web::{get, post, web, Either, Responder};
use futures::future::{join, join_all};
use serde_json::json;
use std::collections::BTreeMap;

#[get("/db/hello/")]
pub async fn hello() -> impl Responder {
    "hello".to_string()
}

#[get("/db/list/")]
pub async fn get_list(
    redis: web::Data<Addr<RedisActor>>,
) -> Either<web::Json<Vec<Tag>>, web::Json<&'static str>> {
    let title_list = match redis_util::get_list("tag", &redis).await {
        Some(v) => v,
        None => {
            return Either::B(web::Json("failed to fetch id list"));
        }
    };

    let tasks = title_list
        .iter()
        .map(|title| redis_util::get_slice(title, "tag", &redis));

    let tasks: Vec<Tag> = join_all(tasks)
        .await
        .iter()
        .map(|slice| {
            let slice = slice.clone().unwrap();
            serde_json::from_slice(&slice).unwrap()
        })
        .collect();

    Either::A(web::Json(tasks))
}

#[get("/db/info/{id}/")]
pub async fn get_vote_info(
    id: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> Either<web::Json<VoteInfo>, web::Json<&'static str>> {
    // let info = VoteInfo::dummy();
    let id = id.into_inner();

    match redis_util::get_slice(&id, "info", &redis).await {
        Some(v) => {
            let info: VoteInfo = serde_json::from_slice(&v).unwrap();
            Either::A(web::Json(info))
        }
        None => Either::B(web::Json("failed to get vote info.")),
    }
}

#[post("/db/info/")]
pub async fn post_vote_info(
    info: web::Json<VoteInfo>,
    redis: web::Data<Addr<RedisActor>>,
) -> Either<web::Json<String>, web::Json<&'static str>> {
    let mut info = info.into_inner();

    // check there is a tag
    let tag_slice = redis_util::get_slice(&info.title, "tag", &redis).await;

    if tag_slice.is_some() {
        let tag: Tag = serde_json::from_slice(&tag_slice.to_owned().unwrap()).unwrap();

        if info.id() == tag.hash {
            return Either::B(web::Json("vote info already saved."));
        }

        info.update_parent(tag.hash);
    }

    let id = info.id();
    let tag = Tag::new(&info.uid, &id, &info.title);
    let add_to_list = redis_util::add(&tag, &redis);
    let add_info_obj = redis_util::add(&info, &redis);

    let (_list, obj) = join(add_to_list, add_info_obj).await;

    match obj {
        true => Either::A(web::Json(info.id())),
        false => Either::B(web::Json("failed to save vote info.")),
    }
}

#[post("/db/info/{uid}/{user}/")]
pub async fn update_user_vote(
    path: web::Path<(String, String)>,
    redis: web::Data<Addr<RedisActor>>,
    vote: web::Json<BTreeMap<String, f64>>,
) -> impl Responder {
    // get the info
    let (uid, user) = path.into_inner();
    let vote = vote.into_inner();

    let tag = redis_util::get_slice(&uid, "tag", &redis).await;

    if tag.is_none() {
        return web::Json("tag not found".to_string());
    }

    let tag: Tag = serde_json::from_slice(&tag.unwrap()).unwrap();

    let info = redis_util::get_slice(&tag.hash, "info", &redis).await;

    if info.is_none() {
        return web::Json("info not found".to_string());
    }

    let mut info: VoteInfo = serde_json::from_slice(&info.unwrap()).unwrap();
    let prev_id = info.id();
    info.params.voters.0.insert(user, vote);

    if prev_id != info.id() {
        let tag = Tag::new(&info.uid, &info.id(), &info.title);
        let add_to_list = redis_util::add(&tag, &redis);
        let add_info_obj = redis_util::add(&info, &redis);

        let (_list, obj) = join(add_to_list, add_info_obj).await;

        return match obj {
            true => web::Json(info.id()),
            false => web::Json("failed to save vote info.".to_string()),
        };
    }

    web::Json("vote info no change".to_string())
}

#[post("/db/info/dummy/")]
pub async fn post_dummy_info(
    redis: web::Data<Addr<RedisActor>>,
) -> Either<web::Json<String>, web::Json<&'static str>> {
    let info = VoteInfo::dummy();

    match redis_util::add(&info, &redis).await {
        true => Either::A(web::Json(info.id())),
        false => Either::B(web::Json("failed to save vote info.")),
    }
}

#[get("/db/info/dummy/")]
pub async fn get_dummy_info() -> web::Json<VoteInfo> {
    let info = VoteInfo::dummy();
    web::Json(info)
}

#[get("/db/result/dummy/")]
pub async fn get_dummy_result() -> impl Responder {
    let result = VoteResult::dummy();
    web::Json(result)
}

#[post("/db/result/dummy/")]
pub async fn post_dummy_result(redis: web::Data<Addr<RedisActor>>) -> impl Responder {
    let result = VoteResult::dummy();
    let _result = redis_util::add(&result, &redis).await;
    web::Json("ok")
}

#[get("/db/result/{info_hash}/")]
pub async fn get_vote_result(
    info_hash: web::Path<String>,
    redis: web::Data<Addr<RedisActor>>,
) -> Either<web::Json<VoteResult>, web::Json<&'static str>> {
    let hash = info_hash.into_inner();
    match redis_util::get_slice(&hash, "result", &redis).await {
        Some(v) => {
            let result: VoteResult = serde_json::from_slice(&v).unwrap();
            Either::A(web::Json(result))
        }
        None => Either::B(web::Json("result not found")),
    }
}

#[post("/db/result/")]
pub async fn post_vote_result(
    redis: web::Data<Addr<RedisActor>>,
    result: web::Json<VoteResult>,
) -> web::Json<&'static str> {
    let result = result.into_inner();
    match redis_util::add(&result, &redis).await {
        true => web::Json("ok"),
        false => web::Json("failed to add result"),
    }
}
