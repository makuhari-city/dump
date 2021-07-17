mod handlers;
mod ipfs;
mod model;
mod redis_object;
mod redis_util;

use actix_cors::Cors;
use actix_redis::RedisActor;
use actix_web::{middleware, web, App, HttpServer};
use dotenv;
use std::env;

pub use redis_object::RedisObject;

use handlers::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    std::env::set_var("RUST_LOG", "actix_web=trace,actix_redis=trace,dump=trace");
    env_logger::init();

    HttpServer::new(|| {
        let redis_addr: String = env::var("REDIS_ADDR").unwrap_or("127.0.0.1".to_string());

        let redis_port: String = env::var("REDIS_PORT").unwrap_or("6379".to_string());

        let address = format!("{}:{}", redis_addr, redis_port);

        let redis_addr = RedisActor::start(&address);

        // TODO: change this
        let cors = Cors::permissive();

        App::new()
            .data(redis_addr)
            .wrap(middleware::Logger::default())
            .wrap(cors)
            .service(
                web::scope("/db")
                    // hello/
                    .service(hello)
                    // * list/
                    .service(get_list)
                    // * rep/ ({repid})
                    .service(get_reps)
                    .service(get_rep)
                    .service(post_rep)
                    .service(get_header)
                    // * history/id/
                    .service(history)
                    // * result/hash/
                    .service(dump_result)
                    .service(get_result)
                    // * topic/update/id/delegate/
                    .service(update_vote)
                    // * topic/update/id/policy/
                    .service(add_policy)
                    // * topic/update/id/field/
                    .service(update_field)
                    // topic/new/
                    .service(make_new_topic)
                    // * topic/raw/hash/
                    .service(get_topic_raw)
                    // * topic/raw/hash/
                    .service(post_topic_raw)
                    // * topic/id/
                    .service(get_topic_by_id),
            )
    })
    .bind("0.0.0.0:8082")?
    .run()
    .await
}
