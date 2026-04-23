use leptos::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ConsentData {
    pub client_name: String,
    pub scope: String,
    pub csrf_token: String,
}

#[server(prefix = "/leptos")]
pub async fn load_consent_data() -> Result<ConsentData, ServerFnError> {
    use kartoteka_db::SqlitePool;
    use kartoteka_oauth::types::PendingOAuthRequest;
    use tower_sessions::Session;

    let session = leptos_axum::extract::<Session>()
        .await
        .map_err(|e| ServerFnError::new(format!("session: {e}")))?;
    let pending: PendingOAuthRequest = session
        .get("pending_oauth_request")
        .await
        .map_err(|e| ServerFnError::new(format!("session get: {e}")))?
        .ok_or_else(|| ServerFnError::new("no pending oauth request".to_string()))?;

    let pool = expect_context::<SqlitePool>();
    let client = kartoteka_db::oauth::clients::find(&pool, &pending.client_id)
        .await
        .map_err(|e| ServerFnError::new(format!("db: {e}")))?
        .ok_or_else(|| ServerFnError::new("unknown client".to_string()))?;

    Ok(ConsentData {
        client_name: client.name,
        scope: pending.scope,
        csrf_token: pending.csrf_token,
    })
}

#[component]
pub fn OAuthConsentPage() -> impl IntoView {
    let data = Resource::new(|| (), |_| load_consent_data());

    view! {
        <div class="min-h-screen bg-base-100 flex items-center justify-center p-4">
            <div class="card w-full max-w-md bg-base-200 shadow-xl">
                <Suspense fallback=|| view!{ <div class="card-body">"Loading…"</div> }>
                    {move || data.get().map(|r| match r {
                        Ok(d) => view!{ <ConsentForm data=d /> }.into_any(),
                        Err(e) => view!{
                            <div class="card-body">
                                <h2 class="card-title text-error">"Error"</h2>
                                <p>{e.to_string()}</p>
                            </div>
                        }.into_any(),
                    })}
                </Suspense>
            </div>
        </div>
    }
}

#[component]
fn ConsentForm(data: ConsentData) -> impl IntoView {
    view! {
        <form method="post" action="/oauth/authorize" class="card-body space-y-4">
            <h2 class="card-title">"Authorize access"</h2>
            <p>
                <strong>{data.client_name.clone()}</strong>
                " wants to access your Kartoteka account."
            </p>
            <div class="bg-base-300 p-3 rounded">
                <p class="text-sm opacity-70">"Permissions requested:"</p>
                <p class="font-mono">{data.scope}</p>
            </div>
            <p class="text-sm opacity-70">
                "Only approve if you trust this application."
            </p>
            <input type="hidden" name="csrf_token" value=data.csrf_token />
            <div class="card-actions justify-end gap-2">
                <button type="submit" name="decision" value="deny" class="btn btn-ghost">
                    "Deny"
                </button>
                <button type="submit" name="decision" value="approve" class="btn btn-primary">
                    "Approve"
                </button>
            </div>
        </form>
    }
}
