// Placeholder for auth handlers — will be fleshed out in Step 6.
// POST /auth/register, POST /auth/login, POST /auth/refresh, POST /auth/logout

use axum::response::IntoResponse;

pub async fn register() -> impl IntoResponse {
    "register placeholder"
}

pub async fn login() -> impl IntoResponse {
    "login placeholder"
}

pub async fn refresh() -> impl IntoResponse {
    "refresh placeholder"
}

pub async fn logout() -> impl IntoResponse {
    "logout placeholder"
}