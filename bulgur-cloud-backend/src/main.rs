use bulgur_cloud::{
    app::setup_app,
    auth::{create_nobody, TOKEN_VALID_SECS},
    cli::{cli_command, Opt},
    folder,
    state::{AppState, PathTokenCache, TokenCache},
};

use clap::StructOpt;

#[cfg(feature = "telemetry_opentelemetry")]
use opentelemetry_otlp::WithExportConfig;
#[cfg(feature = "telemetry_opentelemetry")]
use tonic::metadata::{MetadataKey, MetadataMap};

use std::{env, path::PathBuf, str::FromStr};

use actix_governor::GovernorConfigBuilder;
use actix_web::{web, HttpServer};

use tokio::fs;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_unwrap::OptionExt;

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

    #[cfg(feature = "telemetry_opentelemetry")]
    {
        if env::var("OTEL_SERVICE_NAME").is_ok() {
            const OTEL_HEADER_PREFIX: &'static str = "OTEL_META_";

            let mut meta = MetadataMap::new();
            for (key, value) in env::vars()
                .filter(|(name, _)| name.starts_with(OTEL_HEADER_PREFIX))
                .map(|(name, value)| {
                    let header_name = name
                        .strip_prefix(OTEL_HEADER_PREFIX)
                        .map(|h| h.replace('_', "-"))
                        .map(|h| h.to_ascii_lowercase())
                        .unwrap();
                    (header_name, value)
                })
            {
                meta.insert(MetadataKey::from_str(&key).unwrap(), value.parse().unwrap());
            }

            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .with_env()
                .with_metadata(meta);
            let tracer = opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(exporter)
                .install_batch(opentelemetry::runtime::Tokio)
                .expect("Failed to create opentelemetry tracer");
            let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

            set_global_default(subscriber.with(telemetry)).expect("Failed to set up logging");
        } else {
            set_global_default(subscriber).expect("Failed to set up logging");
        }
    }
    #[cfg(not(feature = "telemetry_opentelemetry"))]
    {
        set_global_default(subscriber).expect("Failed to set up logging");
    }
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
    let opts = Opt::parse();
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

            HttpServer::new(move || setup_app(state.clone(), login_governor.clone()))
                .bind(opts.bind)?
                .workers(opts.workers)
                .run()
                .await?;
            Ok(())
        }
    }
}
