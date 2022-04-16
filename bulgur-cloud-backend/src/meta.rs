use actix_web::{get, head, web, HttpResponse};
use chrono::Duration;
use utoipa::Component;

use crate::state::AppState;
use serde::{Serialize, Serializer};

fn serialize_duration<S>(duration: &chrono::Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_duration = duration.to_string();
    serializer.serialize_str(&formatted_duration)
}

#[derive(Serialize, Component)]
pub struct Stats {
    #[component(example = "PT110.350761445S")]
    #[serde(serialize_with = "serialize_duration")]
    pub uptime: Duration,
}

#[utoipa::path(
    path = "/api/stats",
    responses(
        (status = 200, description = "Server stats, such as the current uptime.", body = Stats),
    ),
)]
#[get("/stats")]
async fn get_stats(state: web::Data<AppState>) -> web::Json<Stats> {
    web::Json(Stats {
        uptime: chrono::Local::now() - state.started_at,
    })
}

#[utoipa::path(
    path = "/api/stats",
    responses(
        (status = 200, description = "Means that the authentication token used is valid. Can be used to validate tokens."),
    ),
)]
#[head("/stats")]
async fn head_stats(_state: web::Data<AppState>) -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[utoipa::path(
    responses(
        (status = 200, description = "Means that the server is in fact a Bulgur Cloud server. Can be used to auto-detect the server."),
    ),
)]
#[head("/is_bulgur_cloud")]
async fn is_bulgur_cloud() -> HttpResponse {
    HttpResponse::Ok().finish()
}
