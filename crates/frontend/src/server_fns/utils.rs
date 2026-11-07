#[cfg(feature = "ssr")]
pub(crate) use kartoteka_shared::date_utils::format_datetime_in_tz;

// ── Server-side helpers shared by every server function ───────────────────────

#[cfg(feature = "ssr")]
use {
    axum_login::AuthSession,
    kartoteka_auth::{KartotekaBackend, KartotekaUser},
    leptos::prelude::*,
    sqlx::SqlitePool,
};

/// The DB pool plus the authenticated user of the current request.
#[cfg(feature = "ssr")]
pub(crate) type AuthedRequest = (SqlitePool, KartotekaUser);

/// The logged-in user of the current request, without touching the DB pool.
///
/// The error messages are part of the client contract — do not reword them.
#[cfg(feature = "ssr")]
pub(crate) async fn current_user() -> Result<KartotekaUser, ServerFnError> {
    let auth = leptos_axum::extract::<AuthSession<KartotekaBackend>>()
        .await
        .map_err(|_| ServerFnError::new("auth extraction failed".to_string()))?;
    auth.user
        .ok_or_else(|| ServerFnError::new("unauthorized".to_string()))
}

/// Authenticate the current request: DB pool + logged-in user.
///
/// Authenticates first and reads the pool with `use_context`, so a server fn
/// run without request context (route-list generation, the static fallback
/// handler) returns an error instead of panicking.
#[cfg(feature = "ssr")]
pub(crate) async fn require_user() -> Result<AuthedRequest, ServerFnError> {
    let user = current_user().await?;
    let pool = use_context::<SqlitePool>()
        .ok_or_else(|| ServerFnError::new("database unavailable".to_string()))?;
    Ok((pool, user))
}

/// Like [`require_user`], but also rejects non-admin users.
#[cfg(feature = "ssr")]
pub(crate) async fn require_admin() -> Result<AuthedRequest, ServerFnError> {
    let (pool, user) = require_user().await?;
    if user.role != "admin" {
        return Err(ServerFnError::new("forbidden".to_string()));
    }
    Ok((pool, user))
}

/// Turns any `Result<T, E: Display>` into a server fn result.
#[cfg(feature = "ssr")]
pub(crate) trait SfnResultExt<T> {
    fn sfn(self) -> Result<T, ServerFnError>;
}

#[cfg(feature = "ssr")]
impl<T, E: std::fmt::Display> SfnResultExt<T> for Result<T, E> {
    fn sfn(self) -> Result<T, ServerFnError> {
        self.map_err(|e| ServerFnError::new(e.to_string()))
    }
}

#[cfg(any(feature = "ssr", test))]
use kartoteka_shared::types::Container;

#[cfg(any(feature = "ssr", test))]
pub(crate) fn build_ancestors(container: &Container, all: &[Container]) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut current_parent = container.parent_container_id.as_deref();
    for _ in 0..10 {
        let Some(pid) = current_parent else { break };
        let Some(parent) = all.iter().find(|c| c.id == pid) else {
            break;
        };
        result.push((format!("/containers/{}", parent.id), parent.name.clone()));
        current_parent = parent.parent_container_id.as_deref();
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::types::Container;

    fn make_container(id: &str, parent_id: Option<&str>) -> Container {
        Container {
            id: id.to_string(),
            user_id: "u1".to_string(),
            name: format!("Name {id}"),
            icon: None,
            color: None,
            description: None,
            status: None,
            parent_container_id: parent_id.map(str::to_string),
            position: 0,
            pinned: false,
            archived: false,
            last_opened_at: None,
            location_id: None,
            created_at: "2024-01-01".to_string(),
            updated_at: "2024-01-01".to_string(),
            ancestors: vec![],
        }
    }

    #[test]
    fn root_container_has_no_ancestors() {
        let root = make_container("a", None);
        let all = vec![root.clone()];
        assert!(build_ancestors(&root, &all).is_empty());
    }

    #[test]
    fn one_level_deep_returns_parent() {
        let parent = make_container("a", None);
        let child = make_container("b", Some("a"));
        let all = vec![parent.clone(), child.clone()];
        let ancestors = build_ancestors(&child, &all);
        assert_eq!(
            ancestors,
            vec![("/containers/a".to_string(), "Name a".to_string())]
        );
    }

    #[test]
    fn two_levels_deep_returns_ordered_path() {
        let root = make_container("a", None);
        let mid = make_container("b", Some("a"));
        let leaf = make_container("c", Some("b"));
        let all = vec![root.clone(), mid.clone(), leaf.clone()];
        let ancestors = build_ancestors(&leaf, &all);
        assert_eq!(
            ancestors,
            vec![
                ("/containers/a".to_string(), "Name a".to_string()),
                ("/containers/b".to_string(), "Name b".to_string()),
            ]
        );
    }

    #[test]
    fn missing_parent_in_all_stops_gracefully() {
        let orphan = make_container("b", Some("nonexistent"));
        let all = vec![orphan.clone()];
        // Should return empty — parent not found, no panic
        assert!(build_ancestors(&orphan, &all).is_empty());
    }
}
