use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use actix_files::NamedFile;
use actix_multipart::{Multipart, MultipartError};
use actix_web::{
    delete, get, head,
    http::{self, StatusCode},
    post, put,
    web::{self, ReqData},
    Either, HttpResponse, HttpResponseBuilder,
};
use futures::TryStreamExt;
use nanoid::nanoid;
use serde::Serialize;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    folder,
    state::{AppState, Authorized, PathTokenResponse, Token},
};

#[cfg(feature = "generate_types")]
use typescript_type_def::TypeDef;

#[derive(Debug, derive_more::Display, thiserror::Error)]
pub enum StorageError {
    #[display(fmt = "User is not authorized to view this.")]
    NotAuthorized,
    #[display(fmt = "Missing token, please log in and get a token.")]
    TokenMissing,
    #[display(fmt = "IO error {}", _0)]
    IOError(#[from] std::io::Error),
    #[display(fmt = "File upload error {}", _0)]
    UploadError(#[from] MultipartError),
    #[display(fmt = "Bad path")]
    BadPath,
}

impl Serialize for StorageError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = format!("{}", self);
        serializer.serialize_str(&s)
    }
}

impl actix_web::error::ResponseError for StorageError {
    fn status_code(&self) -> http::StatusCode {
        match self {
            StorageError::NotAuthorized => StatusCode::UNAUTHORIZED,
            StorageError::TokenMissing => StatusCode::BAD_REQUEST,
            StorageError::IOError(_) => StatusCode::NOT_FOUND, // TODO: This should be set based on what the error is
            StorageError::UploadError(_) => StatusCode::BAD_REQUEST,
            StorageError::BadPath => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponseBuilder::new(self.status_code()).json(&self)
    }
}

/// Gets the path, but only if the user is authorized for it.
pub fn get_authorized_path(
    authorized: &Option<ReqData<Authorized>>,
    store: &str,
) -> Result<PathBuf, StorageError> {
    match authorized {
        Some(authorized) => {
            let is_authorized = match authorized.deref() {
                Authorized::User(user) => store.eq(&user.0),
                Authorized::Path => true,
                Authorized::Both(_) => true,
            };
            if is_authorized {
                tracing::debug!("User or path token is authorized");
                Ok(PathBuf::from(folder::STORAGE).join(store))
            } else {
                tracing::debug!("Not authorized");
                Err(StorageError::NotAuthorized)
            }
        }
        None => {
            tracing::debug!("No auth token attached");
            Err(StorageError::NotAuthorized)
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "generate_types", derive(TypeDef))]
pub struct FolderResults {
    pub entries: Vec<FolderEntry>,
}

#[derive(Serialize)]
#[cfg_attr(feature = "generate_types", derive(TypeDef))]
pub struct FolderEntry {
    pub is_file: bool,
    pub name: String,
    pub size: u64,
}

#[utoipa::path(
    responses(
        (status = 200, description = "If the path is a file, the API will return the file.", body = [NamedFile]),
        (status = 404, description = "The file or folder at this path does not exist.")
    ),
)]
#[get("/{store}/{path:.*}")]
async fn get_storage(
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
) -> Result<Either<NamedFile, web::Json<FolderResults>>, StorageError> {
    let (store, path) = params.as_ref();

    let mut store_path = get_authorized_path(&authorized, store)?;
    store_path.push(path);
    tracing::debug!("Requested path {}", store_path.to_string_lossy());
    if fs::metadata(&store_path).await?.is_file() {
        tracing::debug!("Path is a file");
        Ok(Either::Left(NamedFile::open_async(store_path).await?))
    } else {
        tracing::debug!("Path is a folder");
        let mut entries = fs::read_dir(store_path).await?;
        let mut folder_contents: Vec<FolderEntry> = vec![];
        while let Some(entry) = entries.next_entry().await? {
            let meta = entry.metadata().await?;
            folder_contents.push(FolderEntry {
                is_file: meta.is_file(),
                name: entry.file_name().to_string_lossy().to_string(),
                size: meta.len(),
            })
        }
        Ok(Either::Right(web::Json(FolderResults {
            entries: folder_contents,
        })))
    }
}

fn empty_ok_response() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[tracing::instrument]
#[delete("/{store}/{path:.*}")]
async fn delete_storage(
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
) -> Result<HttpResponse, StorageError> {
    let (store, path) = params.as_ref();

    let mut store_path = get_authorized_path(&authorized, store)?;
    store_path.push(path);
    if fs::metadata(&store_path).await?.is_file() {
        tracing::debug!("Deleting file {:?}", store_path);
        fs::remove_file(&store_path).await?;
    } else {
        tracing::debug!("Deleting folder {:?}", store_path);
        fs::remove_dir_all(&store_path).await?;
    }
    Ok(empty_ok_response())
}

#[tracing::instrument]
#[head("/{store}/{path:.*}")]
async fn head_storage(
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
) -> HttpResponse {
    let (store, path) = params.as_ref();

    let check = async {
        let mut store_path = get_authorized_path(&authorized, store)?;
        store_path.push(path);
        tracing::debug!("Requested path {}", store_path.to_string_lossy());

        let meta = fs::metadata(store_path).await?;
        if meta.is_file() || meta.is_dir() {
            Ok::<(), StorageError>(())
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, ""))?
        }
    }
    .await;

    match check {
        Ok(_) => empty_ok_response(),
        Err(StorageError::NotAuthorized) => HttpResponse::Unauthorized().finish(),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[derive(Debug, Serialize)]
pub struct PutStoragePayload {
    pub files_written: u32,
}

#[put("/{store}/{path:.*}")]
#[tracing::instrument(skip(payload))]
async fn put_storage(
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
    mut payload: Multipart,
) -> Result<web::Json<PutStoragePayload>, StorageError> {
    let (store, path) = params.as_ref();
    let mut store_path = get_authorized_path(&authorized, store)?;
    store_path.push(path);

    tokio::fs::create_dir(&store_path).await.or_else(|err| {
        tracing::debug!("Error: {:?}", err);
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            // It's fine if the folder already exists
            std::io::Result::Ok(())
        } else {
            tracing::warn!("Unexpected error: {:?}", err);
            std::io::Result::Err(err)
        }
    })?;

    match write_files(&mut payload, &store_path).await {
        Ok(files_written) => Ok(web::Json(PutStoragePayload { files_written })),
        Err(err) => Err(err),
    }
}

async fn write_files(payload: &mut Multipart, store_path: &Path) -> Result<u32, StorageError> {
    let mut files_written: u32 = 0;
    while let Some(mut field) = payload.try_next().await? {
        files_written += 1;
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map_or_else(|| nanoid!().to_string(), sanitize_filename::sanitize);
        let filepath = store_path.join(filename);
        tracing::debug!("Uploading path {}", filepath.to_string_lossy());

        let mut file = tokio::fs::File::create(&filepath).await?;
        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            file.write_all(&chunk).await?;
        }
    }
    Ok(files_written)
}

#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "generate_types", derive(TypeDef))]
#[serde(tag = "action")]
pub enum StorageAction {
    MakePathToken,
    Move { new_path: String },
    CreateFolder,
}

#[post("/{store}/{path:.*}")]
async fn post_storage(
    state: web::Data<AppState>,
    action: web::Json<StorageAction>,
    params: web::Path<(String, String)>,
    authorized: Option<ReqData<Authorized>>,
) -> Result<HttpResponse, StorageError> {
    let (store, path) = params.as_ref();
    let mut store_path = get_authorized_path(&authorized, store)?;
    store_path.push(path);
    match action.deref() {
        StorageAction::MakePathToken => Ok(make_path_token(&state, &store_path).await),
        StorageAction::Move { new_path } => {
            let (to_store, to_path) = parse_store_path(new_path).ok_or(StorageError::BadPath)?;
            let mut to_store_path = get_authorized_path(&authorized, to_store)?;
            to_store_path.push(to_path);
            fs::rename(store_path, to_store_path).await?;
            Ok(empty_ok_response())
        }
        StorageAction::CreateFolder => {
            tokio::fs::create_dir(&store_path).await?;
            Ok(empty_ok_response())
        }
    }
}

async fn make_path_token(state: &web::Data<AppState>, path: &Path) -> HttpResponse {
    let token = Token::new();
    let full_path = format!("/{}", path.to_string_lossy());
    tracing::debug!("Creating a token for {}", full_path);
    let mut path_token_cache = state.path_token_cache.0.write().await;
    path_token_cache.insert(full_path, token.clone());
    drop(path_token_cache);
    HttpResponse::Ok().json(PathTokenResponse { token })
}

fn parse_store_path(path: &str) -> Option<(&str, String)> {
    let mut segments = path.split('/').filter(|segment| segment.len() > 0);
    let store = segments.next();
    let rest: Vec<&str> = segments.collect();
    store.map(|store| (store, rest.join("/")))
}
