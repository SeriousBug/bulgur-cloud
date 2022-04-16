use bulgur_cloud::{
    auth::{create_nobody, Login, LoginResponse, Password, TOKEN_VALID_SECS},
    auth_middleware,
    cli::{cli_command, Opt},
    folder,
    meta::{get_stats, head_stats, is_bulgur_cloud, Stats},
    state::{AppState, PathTokenCache, Token, TokenCache},
    storage::{delete_storage, get_storage, head_storage, post_storage, put_storage},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use std::path::PathBuf;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{http, web, App, HttpServer};

use structopt::StructOpt;
use tokio::fs;
use tracing::subscriber::set_global_default;
use tracing_actix_web::TracingLogger;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_unwrap::OptionExt;

#[derive(OpenApi)]
#[openapi(
    handlers(
        bulgur_cloud::auth::login,
        bulgur_cloud::meta::get_stats,
        bulgur_cloud::meta::head_stats,
        bulgur_cloud::meta::is_bulgur_cloud,
        bulgur_cloud::storage::get_storage,
    ),
    components(Login, LoginResponse, Token, Password, Stats)
)]
pub struct ApiDoc;

fn setup_logging() -> () {
    // Wow, thanks Luca Palmieri! https://www.lpalmieri.com/posts/2020-09-27-zero-to-production-4-are-we-observable-yet/

    // We are falling back to printing all spans at info-level or above
    // if the RUST_LOG environment variable has not been set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new(
        "bulgur-cloud".into(),
        // Output the formatted spans to stdout.
        std::io::stdout,
    );
    // The `with` method is provided by `SubscriberExt`, an extension
    // trait for `Subscriber` exposed by `tracing_subscriber`
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    set_global_default(subscriber).expect("Failed to set subscriber");
}

#[cfg(debug_assertions)]
/// During debugging, throttling login attempts is not really needed
const MAX_LOGIN_ATTEMPTS_PER_MIN: u32 = 1000;
#[cfg(not(debug_assertions))]
/// For release, we strictly throttle login attempts to resist brute force attacks
const MAX_LOGIN_ATTEMPTS_PER_MIN: u32 = 10;

/// Store up to this many path tokens. Path tokens are needed to authorize
/// read-only access to specific paths.
const MAX_PATH_TOKEN_CACHE: usize = 100000;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let opts = Opt::from_args();
    match opts.command {
        Some(command) => {
            // Running a CLI command
            cli_command(command).await?;
            Ok(())
        }
        None => {
            // Running the server
            // Make sure the needed folders are available
            fs::create_dir_all(PathBuf::from(folder::STORAGE)).await?;
            let state = web::Data::new(AppState {
                started_at: chrono::Local::now(),
                // Auth tokens are cached for 24 hours
                token_cache: TokenCache::new(TOKEN_VALID_SECS),
                path_token_cache: PathTokenCache::new(MAX_PATH_TOKEN_CACHE),
            });
            // Make sure the nobody user is created if it doesn't exist
            create_nobody().await?;

            setup_logging();

            let login_governor = GovernorConfigBuilder::default()
                .per_second(60)
                .burst_size(MAX_LOGIN_ATTEMPTS_PER_MIN)
                .finish()
                .unwrap_or_log();

            HttpServer::new(move || {
                let cors = Cors::default()
                    .allowed_origin_fn(|origin, _req_head| {
                        origin.as_bytes().starts_with(b"http://localhost")
                    })
                    .allowed_methods(vec!["GET", "POST", "PUT", "HEAD", "DELETE"])
                    .allowed_headers(vec![
                        http::header::AUTHORIZATION,
                        http::header::ACCEPT,
                        http::header::CONTENT_TYPE,
                    ])
                    .max_age(3600);

                let api_guard = auth_middleware::CheckLogin {
                    state: state.clone(),
                    allow_path_tokens: false,
                };
                let storage_guard = auth_middleware::CheckLogin {
                    state: state.clone(),
                    allow_path_tokens: true,
                };

                // Login scope just handles logins. It is heavily throttled to resist brute force attacks.
                let login_scope = web::scope("/auth")
                    .wrap(Governor::new(&login_governor))
                    .service(bulgur_cloud::auth::login);
                // API scope handles all api functionality (anything except storage)
                let api_scope = web::scope("/api")
                    .wrap(api_guard.clone())
                    .service(get_stats)
                    .service(head_stats);
                // Storage scope handles the actual files and folders
                let storage_scope = web::scope("/storage")
                    .wrap(storage_guard.clone())
                    .service(get_storage)
                    .service(put_storage)
                    .service(head_storage)
                    .service(post_storage)
                    .service(delete_storage);
                // Build the app with all these scopes, and add middleware for CORS and tracing
                App::new()
                    .wrap(TracingLogger::default())
                    .wrap(cors)
                    .app_data(state.clone())
                    .service(login_scope)
                    .service(api_scope)
                    .service(storage_scope)
                    .service(is_bulgur_cloud)
                    .service(
                        SwaggerUi::new("/swagger-ui/{_:.*}")
                            .url("/api-doc/openapi.json", ApiDoc::openapi()),
                    )
            })
            .bind("0.0.0.0:8000")?
            .run()
            .await?;
            Ok(())
        }
    }
}
