mod handlers;
mod model;
mod redis_object;
mod redis_util;

use actix_cors::Cors;
use actix_redis::RedisActor;
use actix_web::{middleware, App, HttpServer};
use dotenv;
use model::VoteInfo;
use std::env;

pub use redis_object::RedisObject;

use handlers::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=trace,actix_redis=trace,cityio=debug");
    env_logger::init();

    HttpServer::new(|| {
        let address = format!(
            "{}:{}",
            env::var("REDIS_ADDR").unwrap(),
            env::var("REDIS_PORT").unwrap()
        );

        let redis_addr = RedisActor::start(&address);

        // TODO: change this
        let cors = Cors::permissive();

        App::new()
            .data(redis_addr)
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(hello)
            .service(post_dummy_info)
            .service(get_dummy_info)
            .service(post_dummy_result)
            .service(get_dummy_result)
            .service(get_list)
            .service(get_vote_info)
            .service(post_vote_info)
            .service(update_user_vote)
            .service(get_vote_result)
            .service(post_vote_result)
    })
    .bind("0.0.0.0:8082")?
    .run()
    .await
}
