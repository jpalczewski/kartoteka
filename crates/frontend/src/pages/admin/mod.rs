use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

#[cfg(target_arch = "wasm32")]
use crate::api::client::GlooClient;
use crate::state::AdminContext;

#[component]
pub fn AdminPage() -> impl IntoView {
    let navigate = use_navigate();
    let admin_ctx = use_context::<AdminContext>();

    // Guard: redirect non-admins
    Effect::new(move |_| {
        if let Some(ctx) = admin_ctx {
            if !ctx.is_admin.get() {
                navigate("/", Default::default());
            }
        }
    });

    view! {
        <div class="p-6 max-w-2xl mx-auto space-y-8">
            <h1 class="text-3xl font-bold">{move_tr!("admin-panel")}</h1>
            <InstanceSettingsSection/>
            <InvitationCodesSection/>
        </div>
    }
}

// ── Instance Settings section ──────────────────────────────────────────────

#[component]
fn InstanceSettingsSection() -> impl IntoView {
    let reg_mode = RwSignal::new("open".to_string());
    let saving = RwSignal::new(false);

    #[cfg(target_arch = "wasm32")]
    let _settings_res = {
        let client = use_context::<GlooClient>().expect("GlooClient not provided");
        LocalResource::new(move || {
            let client = client.clone();
            async move {
                if let Ok(settings) = crate::api::admin::list_instance_settings(&client).await {
                    for s in settings {
                        if s.key == kartoteka_shared::INSTANCE_SETTING_REGISTRATION_MODE {
                            if let Some(m) = s.value.as_str() {
                                reg_mode.set(m.to_string());
                            }
                        }
                    }
                }
            }
        })
    };

    let on_save = move |_| {
        saving.set(true);
        #[cfg(target_arch = "wasm32")]
        let mode = reg_mode.get_untracked();
        leptos::task::spawn_local(async move {
            #[cfg(target_arch = "wasm32")]
            {
                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                let _ = crate::api::admin::update_instance_setting(
                    &client,
                    kartoteka_shared::INSTANCE_SETTING_REGISTRATION_MODE,
                    serde_json::Value::String(mode),
                )
                .await;
            }
            saving.set(false);
        });
    };

    view! {
        <section class="card bg-base-200 border border-base-300">
            <div class="card-body">
                <h2 class="card-title">{move_tr!("instance-settings")}</h2>
                <div class="form-control">
                    <label class="label"><span class="label-text">{move_tr!("registration-mode")}</span></label>
                    <select
                        class="select select-bordered w-full max-w-xs"
                        on:change=move |ev| reg_mode.set(event_target_value(&ev))
                        prop:value=move || reg_mode.get()
                    >
                        <option value="open">{move_tr!("registration-open")}</option>
                        <option value="invite">{move_tr!("registration-invite")}</option>
                        <option value="closed">{move_tr!("registration-closed")}</option>
                    </select>
                </div>
                <div class="card-actions justify-end mt-4">
                    <button
                        class="btn btn-primary"
                        disabled=move || saving.get()
                        on:click=on_save
                    >
                        {move_tr!("common-save")}
                    </button>
                </div>
            </div>
        </section>
    }
}

// ── Invitation Codes section ───────────────────────────────────────────────

#[component]
fn InvitationCodesSection() -> impl IntoView {
    let codes = RwSignal::new(Vec::<kartoteka_shared::InvitationCode>::new());
    let generating = RwSignal::new(false);

    #[cfg(target_arch = "wasm32")]
    let _codes_res = {
        let client = use_context::<GlooClient>().expect("GlooClient not provided");
        LocalResource::new(move || {
            let client = client.clone();
            async move {
                if let Ok(list) = crate::api::admin::list_invitation_codes(&client).await {
                    codes.set(list);
                }
            }
        })
    };

    let on_generate = move |_| {
        generating.set(true);
        leptos::task::spawn_local(async move {
            #[cfg(target_arch = "wasm32")]
            {
                let client = use_context::<GlooClient>().expect("GlooClient not provided");
                match crate::api::admin::create_invitation_code(&client, None).await {
                    Ok(new_code) => codes.update(|list| list.insert(0, new_code)),
                    Err(_) => {}
                }
            }
            generating.set(false);
        });
    };

    view! {
        <section class="card bg-base-200 border border-base-300">
            <div class="card-body">
                <div class="flex justify-between items-center">
                    <h2 class="card-title">{move_tr!("invitation-codes")}</h2>
                    <button
                        class="btn btn-primary btn-sm"
                        disabled=move || generating.get()
                        on:click=on_generate
                    >
                        {move_tr!("generate-code")}
                    </button>
                </div>
                <div class="overflow-x-auto mt-4">
                    <table class="table table-sm">
                        <thead>
                            <tr>
                                <th>{move_tr!("invite-code")}</th>
                                <th>{move_tr!("common-status")}</th>
                                <th>{move_tr!("common-created")}</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            {move || codes.get().into_iter().map(|c| {
                                let id_del = c.id.clone();
                                let dimmed = c.used_by.is_some();
                                // Inline delete callback — Fn-compatible (codes is Copy RwSignal)
                                let delete_this = move |_: web_sys::MouseEvent| {
                                    #[allow(unused_variables)]
                                    let id = id_del.clone();
                                    leptos::task::spawn_local(async move {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let client = use_context::<GlooClient>().expect("GlooClient not provided");
                                            if crate::api::admin::delete_invitation_code(&client, &id).await.is_ok() {
                                                codes.update(|list| list.retain(|item| item.id != id));
                                            }
                                        }
                                    });
                                };
                                let delete_btn = if !dimmed {
                                    view! {
                                        <button
                                            class="btn btn-ghost btn-xs text-error"
                                            on:click=delete_this
                                        >
                                            "✕"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! { <span></span> }.into_any()
                                };
                                view! {
                                    <tr class=if dimmed { "opacity-50" } else { "" }>
                                        <td><code class="font-mono">{c.code.clone()}</code></td>
                                        <td>{if dimmed { move_tr!("code-used").get() } else { move_tr!("code-active").get() }}</td>
                                        <td class="text-xs">{c.created_at.clone()}</td>
                                        <td>{delete_btn}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                </div>
            </div>
        </section>
    }
}
