use kartoteka_shared::{FEATURE_DEADLINES, FEATURE_QUANTITY, ListFeature};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::editable_title::EditableTitle;

#[component]
pub fn ListHeader(
    list_name: String,
    list_id: String,
    item_count: usize,
    on_delete_confirmed: Callback<()>,
    #[prop(optional)] on_archive: Option<Callback<()>>,
    #[prop(optional)] on_reset: Option<Callback<()>>,
    #[prop(optional)] on_rename: Option<Callback<String>>,
    #[prop(default = vec![])] features: Vec<ListFeature>,
    #[prop(optional)] on_feature_toggle: Option<Callback<(String, bool)>>,
    /// Called when deadlines sub-config changes: new full config JSON
    #[prop(optional)]
    on_deadlines_config_change: Option<Callback<serde_json::Value>>,
) -> impl IntoView {
    let show_delete = RwSignal::new(false);
    let show_settings = RwSignal::new(false);
    let has_quantity = features.iter().any(|f| f.name == FEATURE_QUANTITY);
    let has_deadlines = features.iter().any(|f| f.name == FEATURE_DEADLINES);
    let deadlines_config = features
        .iter()
        .find(|f| f.name == FEATURE_DEADLINES)
        .map(|f| f.config.clone())
        .unwrap_or(serde_json::json!({}));
    let cfg_has_start = deadlines_config
        .get("has_start_date")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let cfg_has_deadline = deadlines_config
        .get("has_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let cfg_has_hard = deadlines_config
        .get("has_hard_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let start_checked = RwSignal::new(cfg_has_start);
    let deadline_checked = RwSignal::new(cfg_has_deadline);
    let hard_checked = RwSignal::new(cfg_has_hard);

    let fire_config_change = move || {
        if let Some(cb) = on_deadlines_config_change {
            let config = serde_json::json!({
                "has_start_date": start_checked.get(),
                "has_deadline": deadline_checked.get(),
                "has_hard_deadline": hard_checked.get(),
            });
            cb.run(config);
        }
    };

    view! {
        <div class="flex flex-col gap-3 mb-4 sm:flex-row sm:items-start sm:justify-between">
            <div class="min-w-0 flex-1">
                {if let Some(on_rename) = on_rename {
                    view! {
                        <EditableTitle
                            value=list_name.clone()
                            on_save=on_rename
                            class="text-2xl font-bold block break-words".to_string()
                        />
                    }.into_any()
                } else {
                    view! { <h2 class="text-2xl font-bold break-words">{list_name.clone()}</h2> }.into_any()
                }}
            </div>
            <div class="flex flex-wrap gap-1 sm:justify-end">
                {on_feature_toggle.map(|_| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| show_settings.update(|v| *v = !*v)
                    >
                        {move_tr!("lists-header-settings-button")}
                    </button>
                })}
                {on_reset.map(|cb| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| cb.run(())
                    >
                        {move_tr!("lists-header-reset-button")}
                    </button>
                })}
                {on_archive.map(|cb| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| cb.run(())
                    >
                        {move_tr!("lists-header-archive-button")}
                    </button>
                })}
                <button
                    type="button"
                    class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                    on:click=move |_| show_delete.set(true)
                >
                    {move_tr!("lists-header-delete-button")}
                </button>
            </div>
        </div>

        // Feature settings panel
        {move || {
            if show_settings.get() {
                on_feature_toggle.map(|on_toggle| view! {
                    <div class="bg-base-200 rounded-lg p-3 mb-4">
                        <div class="flex items-center gap-4">
                            <span class="text-sm font-semibold">{move_tr!("lists-features-label")}</span>
                            <label class="label cursor-pointer gap-2">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-sm"
                                    prop:checked=has_quantity
                                    on:change=move |ev| {
                                        on_toggle.run((FEATURE_QUANTITY.to_string(), event_target_checked(&ev)));
                                    }
                                />
                                <span class="label-text">{move_tr!("lists-feature-quantities")}</span>
                            </label>
                            <label class="label cursor-pointer gap-2">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-sm"
                                    prop:checked=has_deadlines
                                    on:change=move |ev| {
                                        on_toggle.run((FEATURE_DEADLINES.to_string(), event_target_checked(&ev)));
                                    }
                                />
                                <span class="label-text">{move_tr!("lists-feature-deadlines")}</span>
                            </label>
                        </div>
                        // Deadlines sub-config
                        {if has_deadlines {
                            view! {
                                <div class="flex items-center gap-4 mt-2 ml-4 text-xs">
                                    <span class="opacity-50">{move_tr!("lists-deadlines-dates-label")}</span>
                                    <label class="label cursor-pointer gap-1">
                                        <input
                                            type="checkbox"
                                            class="checkbox checkbox-xs"
                                            prop:checked=start_checked
                                            on:change=move |ev| {
                                                start_checked.set(event_target_checked(&ev));
                                                fire_config_change();
                                            }
                                        />
                                        <span class="label-text text-xs">{move_tr!("lists-deadlines-start")}</span>
                                    </label>
                                    <label class="label cursor-pointer gap-1">
                                        <input
                                            type="checkbox"
                                            class="checkbox checkbox-xs"
                                            prop:checked=deadline_checked
                                            on:change=move |ev| {
                                                deadline_checked.set(event_target_checked(&ev));
                                                fire_config_change();
                                            }
                                        />
                                        <span class="label-text text-xs">{move_tr!("lists-deadlines-deadline")}</span>
                                    </label>
                                    <label class="label cursor-pointer gap-1">
                                        <input
                                            type="checkbox"
                                            class="checkbox checkbox-xs"
                                            prop:checked=hard_checked
                                            on:change=move |ev| {
                                                hard_checked.set(event_target_checked(&ev));
                                                fire_config_change();
                                            }
                                        />
                                        <span class="label-text text-xs">{move_tr!("lists-deadlines-hard")}</span>
                                    </label>
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                })
            } else {
                None
            }
        }}

        // Delete confirmation modal
        {move || {
            if show_delete.get() {
                let lid = list_id.clone();
                let lname = list_name.clone();
                Some(view! {
                    <ConfirmDeleteModal
                        list_id=lid
                        list_name=lname
                        item_count=item_count
                        on_confirm=Callback::new(move |_| {
                            on_delete_confirmed.run(());
                        })
                        on_cancel=Callback::new(move |_| show_delete.set(false))
                    />
                })
            } else {
                None
            }
        }}
    }
}
