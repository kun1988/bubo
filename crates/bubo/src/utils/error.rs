use axum::{extract::rejection::{FormRejection, JsonRejection, PathRejection, QueryRejection}, http::StatusCode, response::{IntoResponse, Response}, Json};
use axum_extra::extract::JsonDeserializerRejection;
use fred::error::RedisError;
use sea_orm::DbErr;
use serde_json::{json, Value};
use thiserror::Error;
use tracing::{error, warn};

pub type BuboResult<T> = Result<T, BuboError>;

#[derive(Debug, Error)]
pub enum BuboError {
    #[error(transparent)]
    OtherError(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("{1}")]
    SystemError(SystemErrorCode, String),
    #[error("{1}")]
    BusinessError(BusinessErrorCode, String),
    #[error(transparent)]
    ValidationError(#[from] validator::ValidationErrors),
    #[error(transparent)]
    FormRejectionError(#[from] FormRejection),
    #[error(transparent)]
    DatabaseError(#[from] DbErr),
    #[error(transparent)]
    RedisError(#[from] RedisError),
    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),
    #[error(transparent)]
    JsonRejectionError(#[from] JsonRejection),
    #[error(transparent)]
    QueryRejectionError(#[from] QueryRejection),
    #[error(transparent)]
    PathRejectionError(#[from] PathRejection),
    #[error(transparent)]
    JsonDeserializerRejectionError(#[from] JsonDeserializerRejection),
    #[error(transparent)]
    PasswordHashError(#[from] argon2::password_hash::Error),
    
}

impl BuboError {
    pub fn system_error(error_code: SystemErrorCode, error_message: impl Into<String>) -> BuboError {
        BuboError::SystemError(error_code, error_message.into())
    }
    pub fn business_error(error_code: BusinessErrorCode, error_message: impl Into<String>) -> BuboError {
        BuboError::BusinessError(error_code, error_message.into())
    }
}

#[derive(Debug)]
pub enum SystemErrorCode {
    UnknownError = 20000,
    InternalServerError,
    NotFound,
    RequestTimeout,
    ServiceUnavailable,
    DatabaseError,
    RedisError,
    SerdeJsonError,
    JwtEncodeError,
    Argon2HashError,
}
    

#[derive(Debug)]
pub enum BusinessErrorCode {
    UnknownError = 10000,
    ValidationError,
    FormRejectionError,
    JsonRejectionError,
    QueryRejectionError,
    PathRejectionError,
    PasswordHashError,
    AuthFailed,
    Unauthorized,
    Forbidden,
    AlreadyExists,
    NotFound,
    PasswordNotMatch,
    UserOrPasswordNotMatch,
}

impl BuboError {
    pub fn into_error_response(self) -> (StatusCode, Value) {
        let (status, error_code, error_message) = match self {
            BuboError::OtherError(_) => {
                error!("Other error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, SystemErrorCode::UnknownError as usize, "Internal Server Error".to_owned())
            },
            BuboError::SystemError(error_code, error_message) => {
                error!("System error: {}", error_message.as_str());
                (StatusCode::INTERNAL_SERVER_ERROR, error_code as usize, "Internal Server Error".to_owned())
            },
            BuboError::BusinessError(error_code, error_message) => {
                // warn!("Business error: {}", error_message.as_str());
                (StatusCode::OK, error_code as usize, error_message)
            },
            BuboError::ValidationError(_) => {
                // let message = format!("Input validation error: [{self}]").replace('\n', ", ");
                let message = format!("{self}").replace('\n', ", ");
                // let message = "Input validation error".to_owned();
                (StatusCode::OK, BusinessErrorCode::ValidationError as usize, message)
            },
            BuboError::FormRejectionError(r) => {
                // warn!("Form rejection error: {}", r.body_text());
                (StatusCode::OK, BusinessErrorCode::FormRejectionError as usize, r.body_text())
            },
            BuboError::JsonRejectionError(r) => {
                // warn!("Json rejection error: {}", r.body_text());
                (StatusCode::OK, BusinessErrorCode::JsonRejectionError as usize, r.body_text())
            },
            BuboError::QueryRejectionError(r) => {
                // warn!("Query rejection error: {}", r.body_text());
                (StatusCode::OK, BusinessErrorCode::QueryRejectionError as usize, r.body_text())
            },
            BuboError::PathRejectionError(r) => {
                // warn!("Path rejection error: {}", r.body_text());
                (StatusCode::OK, BusinessErrorCode::PathRejectionError as usize, r.body_text())
            },
            BuboError::JsonDeserializerRejectionError(r) => {
                // warn!("Json deserializer rejection error: {}", r.body_text());
                (StatusCode::OK, BusinessErrorCode::PathRejectionError as usize, r.body_text())
            },
            BuboError::PasswordHashError(_) => {
                warn!("Password hash  error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, BusinessErrorCode::PasswordHashError as usize, "Internal Server Error".to_owned())
            },
            BuboError::DatabaseError(_) => {
                error!("Database error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, SystemErrorCode::DatabaseError as usize, "Internal Server Error".to_owned())
            },
            BuboError::RedisError(_) => {
                error!("Redis error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, SystemErrorCode::RedisError as usize, "Internal Server Error".to_owned())
            },
            BuboError::SerdeJsonError(_) => {
                error!("Serde json error: {:?}", self);
                (StatusCode::INTERNAL_SERVER_ERROR, SystemErrorCode::SerdeJsonError as usize, "Internal Server Error".to_owned())
            },
            
        };
        (
            status,
            json!({
                "status": false,
                "error_code": error_code,
                "error_message": error_message,
            }),
        )
    }
}

impl IntoResponse for BuboError {
    fn into_response(self) -> Response {
        let (status, value) = self.into_error_response();
        let body = Json(value);
        (status, body).into_response()
    }
}

