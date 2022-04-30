use std::path::PathBuf;

use actix_web::{
    get,
    http::header::{self, HeaderValue},
    web, HttpResponse,
};
use rust_embed::RustEmbed;
use tracing_unwrap::ResultExt;

fn extension_to_content_type(path: &str) -> Option<&'static str> {
    let path = PathBuf::from(path);
    let extension = path.extension().and_then(|extension| extension.to_str());
    if let Some(extension) = extension {
        return match extension {
            "css" => Some("text/css"),
            "svg" => Some("image/svg+xml"),
            "js" => Some("application/javascript"),
            "json" => Some("application/json"),
            "map" => Some("application/json"),
            "png" => Some("image/png"),
            "jpg" => Some("image/jpeg"),
            "jpeg" => Some("image/jpeg"),
            "html" => Some("text/html"),
            "ico" => Some("image/x-icon"),
            "ttf" => Some("font/ttf"),
            _ => None,
        };
    }
    None
}

type EmbeddedFile = (Vec<u8>, Option<HeaderValue>);

async fn get_by_path<T: RustEmbed>(path: &str, cache: EmbeddedFileCache) -> HttpResponse {
    let cache_read = cache.read().await;
    let embedded_file = match cache_read.get(path) {
        Some(embedded_file) => Some(embedded_file.clone()),
        None => {
            // If the file is not in cache, we need to check if it's embedded in the binary then
            let file = T::get(path);
            match file {
                Some(file) => {
                    // We need a write lock to put the file in the cache.
                    // Dropping the read lock and acquiring the write lock is
                    // safe, because even if another thread wrote the same file
                    // the results should be identical for this thread
                    drop(cache_read);
                    let header_value = extension_to_content_type(path).and_then(|content_type| {
                        Some(HeaderValue::from_str(content_type).unwrap_or_log())
                    });
                    let embedded_file: EmbeddedFile = (file.data.to_vec(), header_value);
                    let mut cache_write = cache.write().await;
                    cache_write.insert(path.to_string(), embedded_file.clone());
                    drop(cache_write);
                    Some(embedded_file)
                }
                None => None,
            }
        }
    };
    embedded_file.map_or_else(
        || HttpResponse::NotFound().finish(),
        |(file, content_type)| {
            let mut response = HttpResponse::Ok().body(file);
            content_type.and_then(|content_type| {
                response
                    .headers_mut()
                    .append(header::CONTENT_TYPE, content_type);
                Some(())
            });
            response
        },
    )
}

type EmbeddedFileCache = tokio::sync::RwLock<std::collections::HashMap<String, EmbeddedFile>>;
fn new_embedded_file_cache() -> EmbeddedFileCache {
    tokio::sync::RwLock::new(std::collections::HashMap::new())
}

lazy_static! {
    static ref UI_CACHE: EmbeddedFileCache = new_embedded_file_cache();
    static ref BASIC_CACHE: EmbeddedFileCache = new_embedded_file_cache();
}

/// Serves the web UI.
#[derive(RustEmbed)]
#[folder = "../bulgur-cloud-frontend/web-build/"]
struct UI;

#[tracing::instrument]
#[get("/")]
pub async fn get_ui_index() -> HttpResponse {
    get_by_path::<UI>("index.html").await
}

#[tracing::instrument]
#[get("/{path:.*}")]
pub async fn get_ui(params: web::Path<String>) -> HttpResponse {
    get_by_path::<UI>(params.as_str()).await
}

/// Serves the static assets required for the basic web UI.
#[derive(RustEmbed)]
#[folder = "assets/"]
struct Basic;

#[tracing::instrument]
#[get("/basic/assets/{path:.*}")]
pub async fn get_basic_assets(params: web::Path<String>) -> HttpResponse {
    get_by_path::<Basic>(params.as_str()).await
}
