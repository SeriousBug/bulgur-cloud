use std::{ops::Deref, path::PathBuf};

use actix_files::NamedFile;
use actix_web::{
    cookie::Cookie,
    get, post,
    web::{self, ReqData},
    Either, HttpResponse,
};

use askama_actix::Template;
use serde::{Deserialize, Serialize};
use tracing_unwrap::ResultExt;

use crate::{
    auth::{verify_pass, Password},
    auth_middleware::AUTH_COOKIE_NAME,
    state::{self, AppState, Authorized, Token},
    storage::{get_storage_internal, FolderEntry, StorageError},
};

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginPage {}

#[tracing::instrument]
#[get("/basic/")]
pub async fn page_login_get() -> LoginPage {
    LoginPage {}
}

#[derive(Template)]
#[template(path = "not-found.html")]
pub struct NotFoundPage {}

#[tracing::instrument]
pub async fn not_found() -> HttpResponse {
    let page = (NotFoundPage {}).render().unwrap_or_log();
    HttpResponse::NotFound().body(page)
}

#[derive(Serialize, Deserialize)]
pub struct LoginFormData {
    pub username: String,
    pub password: Password,
}

#[tracing::instrument(skip(form))]
#[post("/basic/")]
pub async fn page_login_post(
    form: web::Form<LoginFormData>,
    state: web::Data<AppState>,
) -> HttpResponse {
    if verify_pass(&form.username, &form.password).await.is_ok() {
        let mut cache = state.token_cache.0.write().await;
        // generate and cache token
        let token = Token::new();
        // Impossibly unlikely, but token collisions would be extremely bad so check it anyway
        assert!(!cache.contains_key(&token));
        cache.insert(token.clone(), state::User(form.username.clone()));

        HttpResponse::SeeOther()
            .cookie(Cookie::new(
                AUTH_COOKIE_NAME,
                format!("{}; SameSite=Strict", token.reveal()),
            ))
            .append_header(("Location", format!("/basic/{}/", form.username)))
            .finish()
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

#[tracing::instrument]
#[post("/basic/logout")]
pub async fn page_logout() -> HttpResponse {
    let mut remove_cookie = Cookie::named(AUTH_COOKIE_NAME);
    remove_cookie.make_removal();

    HttpResponse::SeeOther()
        .cookie(remove_cookie)
        .append_header(("Location", "/"))
        .finish()
}

#[derive(Template)]
#[template(path = "folder-list.html")]
pub struct FolderListPage {
    username: String,
    path: String,
    folder_list: Vec<FolderEntry>,
}

#[tracing::instrument]
#[get("/{store}/{path:.*}")]
pub async fn page_folder_list(
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
    // TODO: Add a new error type with an HTML responder here
) -> Result<Either<NamedFile, FolderListPage>, StorageError> {
    let (store, path) = params.clone();
    let mut store_path = PathBuf::from(&store);
    if !(&path).is_empty() {
        store_path.push(&path);
    }
    tracing::debug!("{:?}, {:?}, {:?}", &store, &path, &store_path);

    let out = get_storage_internal(params, &authorized).await?;

    match out {
        Either::Left(file) => Ok(Either::Left(file)),
        Either::Right(folder_list) => {
            let username = match &authorized {
                Some(user) => match user.deref() {
                    Authorized::User(user) => user.0.as_str(),
                    Authorized::Path => "anonymous",
                    Authorized::Both(user) => user.0.as_str(),
                },
                None => {
                    return Err(StorageError::NotAuthorized);
                }
            }
            .to_string();
            Ok(Either::Right(FolderListPage {
                username,
                // TODO This is the right thing to do I guess? Not sure how else to handle "bad" characters if they exist
                path: store_path.to_string_lossy().to_string(),
                folder_list: folder_list.0.entries,
            }))
        }
    }
}
