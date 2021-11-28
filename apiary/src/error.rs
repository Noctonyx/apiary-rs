use rafx::api::RafxError;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum ApiaryError {
    StringError(String),
    IoError(Arc<std::io::Error>),
    RafxError(RafxError),
}

impl From<RafxError> for ApiaryError {
    fn from(e: RafxError) -> Self {
        ApiaryError::RafxError(e)
    }
}

pub type ApiaryResult<T> = Result<T, ApiaryError>;
