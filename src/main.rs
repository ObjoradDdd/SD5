mod extractors;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use extractors::{AuthUser, Validate, ValidatedJson};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub environment: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserPayload {
    pub username: String,
    pub email: String,
    pub age: u8,
}

impl Validate for CreateUserPayload {
    fn validate(&self) -> Result<(), String> {
        if self.username.trim().is_empty() {
            return Err("username cannot be empty".to_string());
        }
        if !self.email.contains('@') {
            return Err("email must contain '@'".to_string());
        }
        if self.age < 18 {
            return Err("age must be at least 18".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub status: String,
    pub user_id: String,
    pub username: String,
    pub email: String,
    pub age: u8,
    pub environment: String,
}

async fn create_user(
    State(state): State<AppState>,
    user: AuthUser,
    ValidatedJson(payload): ValidatedJson<CreateUserPayload>,
) -> impl IntoResponse {
    (
        StatusCode::CREATED,
        Json(UserResponse {
            status: "created".to_string(),
            user_id: user.user_id,
            username: payload.username,
            email: payload.email,
            age: payload.age,
            environment: state.environment,
        }),
    )
}

async fn get_profile(user: AuthUser) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "authenticated",
            "user_id": user.user_id,
            "token": user.token
        })),
    )
}

async fn health_check() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

#[tokio::main]
async fn main() {
    let state = AppState {
        environment: "research-sandbox".to_string(),
    };

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/profile", get(get_profile))
        .route("/api/users", post(create_user))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
