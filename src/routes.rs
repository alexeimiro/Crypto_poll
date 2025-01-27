// routes.rs
use actix_web::web;
use crate::handlers;

pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/coins", web::get().to(handlers::get_coins))
            .route("/poll", web::get().to(handlers::get_poll))
            .route("/vote", web::post().to(handlers::vote))
            .route("/admin/select-coins", web::post().to(handlers::select_coins))
    );
}