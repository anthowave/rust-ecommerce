// Placeholder for user handlers — will be fleshed out in Step 6.
// GET /users/me, PUT /users/me, GET /users/:id

use axum::response::IntoResponse;

pub async fn get_me() -> impl IntoResponse {
    "get_me placeholder"
}

pub async fn update_me() -> impl IntoResponse {
    "update_me placeholder"
}

pub async fn get_user() -> impl IntoResponse {
    "get_user placeholder"
}
