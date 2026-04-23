use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::{KartotekaBackend, LoginCredentials},
    kartoteka_domain as domain,
    sqlx::SqlitePool,
};

/// Current user's display name — used by the navbar.
#[server(prefix = "/leptos")]
pub async fn get_nav_data() -> Result<String, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    let user = auth
        .user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))?;
    Ok(user.name.unwrap_or(user.email))
}

/// Authenticate with email + password. On success, sets session cookie and redirects to
/// `return_to` (if it's a safe relative path) or `/`.
#[server(prefix = "/leptos")]
pub async fn do_login(
    email: String,
    password: String,
    return_to: Option<String>,
) -> Result<(), ServerFnError> {
    let mut auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;

    let credentials = LoginCredentials { email, password };
    let user = auth
        .authenticate(credentials)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .ok_or_else(|| ServerFnError::new("invalid credentials".to_string()))?;

    auth.login(&user)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let redirect_to = return_to
        .filter(|r| r.starts_with('/') && !r.starts_with("//"))
        .unwrap_or_else(|| "/".to_string());
    leptos_axum::redirect(&redirect_to);
    Ok(())
}

/// Clear the session and redirect to /login.
#[server(prefix = "/leptos")]
pub async fn do_logout() -> Result<(), ServerFnError> {
    let mut auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;

    auth.logout()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    leptos_axum::redirect("/login");
    Ok(())
}

/// Register a new account. Redirects to /login on success.
#[server(prefix = "/leptos")]
pub async fn do_register(
    email: String,
    password: String,
    name: Option<String>,
) -> Result<(), ServerFnError> {
    let pool = expect_context::<SqlitePool>();
    domain::auth::register(&pool, &email, &password, name.as_deref())
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    leptos_axum::redirect("/login");
    Ok(())
}
