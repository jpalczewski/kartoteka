use kartoteka_shared::models::search::SearchEntityResult;
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_user},
    kartoteka_domain as domain,
};

#[server(prefix = "/leptos")]
pub async fn search_items(query: String) -> Result<Vec<SearchEntityResult>, ServerFnError> {
    let (pool, user) = require_user().await?;

    domain::search::search(&pool, &user.id, &query).await.sfn()
}
