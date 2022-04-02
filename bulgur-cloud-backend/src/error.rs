use std::{fmt::Debug, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub(crate) enum CLIError {
    #[error("Bad user type, should be user or admin")]
    BadUserType(String),
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ServerError {
    #[error("The user data has been modified or corrupted")]
    BadUserData {
        file: PathBuf,
        expected_user: String,
    },
}
