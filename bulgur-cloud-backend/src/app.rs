use crate::{
    auth::login,
    auth_middleware,
    meta::{get_stats, head_stats, is_bulgur_cloud},
    pages::{not_found, page_folder_list, page_login_get, page_login_post, page_logout},
    state::AppState,
    static_files::{get_basic_assets, get_ui, get_ui_index},
    storage::{delete_storage, get_storage, head_storage, post_storage, put_storage},
};

use actix_service::ServiceFactory;

use actix_cors::Cors;
use actix_governor::{Governor, GovernorConfig, PeerIpKeyExtractor};

use actix_web::{
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    http, middleware,
    web::{self, Data},
    App, Error,
};

use tracing_actix_web::TracingLogger;

pub fn setup_app(
    state: Data<AppState>,
    login_governor: GovernorConfig<PeerIpKeyExtractor>,
) -> App<
    impl ServiceFactory<
        ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Config = (),
        InitError = (),
        Error = Error,
    >,
> {
    let cors = Cors::default()
        .allow_any_origin()
        .allowed_methods(vec![
            http::Method::OPTIONS,
            http::Method::GET,
            http::Method::POST,
            http::Method::PUT,
            http::Method::HEAD,
            http::Method::DELETE,
        ])
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
        .service(login);
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
    // Basic HTML scopes are for the javascript-free basic interface.
    let authenticated_basic_html_scope = web::scope("/basic")
        .wrap(storage_guard)
        .service(page_folder_list);
    let basic_html_scope = web::scope("")
        .service(page_login_get)
        .service(page_login_post)
        .service(page_logout)
        .service(get_basic_assets)
        .service(authenticated_basic_html_scope)
        .service(get_ui_index)
        .service(get_ui); /*  */

    // Build the app with all these scopes, and add middleware for CORS and tracing
    App::new()
        .wrap(TracingLogger::default())
        // Fix redundant slashes
        .wrap(middleware::NormalizePath::new(
            middleware::TrailingSlash::MergeOnly,
        ))
        // Negotiate and do compression
        .wrap(middleware::Compress::default())
        // Allow CORS access.
        .wrap(cors)
        .app_data(state.clone())
        .service(login_scope)
        .service(api_scope)
        .service(storage_scope)
        .service(is_bulgur_cloud)
        .service(basic_html_scope)
        .default_service(web::to(not_found))
}
