use axum::{
    async_trait,
    extract::{rejection::JsonRejection, FromRequest, FromRequestParts, Request},
    http::{header, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthUser {
    pub token: String,
    pub user_id: String,
}

#[derive(Debug)]
pub enum AuthRejection {
    MissingHeader,
    InvalidHeader,
    InvalidScheme,
    InvalidToken,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingHeader => (
                StatusCode::UNAUTHORIZED,
                "missing 'Authorization' header",
            ),
            Self::InvalidHeader => (
                StatusCode::UNAUTHORIZED,
                "invalid 'Authorization' header characters",
            ),
            Self::InvalidScheme => (
                StatusCode::UNAUTHORIZED,
                "authorization scheme must be 'Bearer'",
            ),
            Self::InvalidToken => (
                StatusCode::UNAUTHORIZED,
                "token is empty or rejected",
            ),
        };

        (
            status,
            [(header::WWW_AUTHENTICATE, "Bearer")],
            Json(serde_json::json!({
                "error": "unauthorized",
                "message": message
            })),
        )
            .into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(header::AUTHORIZATION)
            .ok_or(AuthRejection::MissingHeader)?;

        let header_str = header_value
            .to_str()
            .map_err(|_| AuthRejection::InvalidHeader)?;

        let token = header_str
            .strip_prefix("Bearer ")
            .ok_or(AuthRejection::InvalidScheme)?;

        if token.is_empty() || token == "blocked-token" {
            return Err(AuthRejection::InvalidToken);
        }

        Ok(AuthUser {
            token: token.to_string(),
            user_id: format!("usr_{}", token),
        })
    }
}

pub trait Validate {
    fn validate(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ValidatedJson<T>(pub T);

impl<T> Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub enum ValidatedJsonRejection {
    Json(JsonRejection),
    Validation(String),
}

impl IntoResponse for ValidatedJsonRejection {
    fn into_response(self) -> Response {
        match self {
            Self::Json(err) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "malformed_json",
                    "message": err.body_text()
                })),
            )
                .into_response(),
            Self::Validation(err) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "validation_failed",
                    "message": err
                })),
            )
                .into_response(),
        }
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate,
    S: Send + Sync,
{
    type Rejection = ValidatedJsonRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(ValidatedJsonRejection::Json)?;

        value
            .validate()
            .map_err(ValidatedJsonRejection::Validation)?;

        Ok(ValidatedJson(value))
    }
}
