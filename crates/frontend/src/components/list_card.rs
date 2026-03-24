use kartoteka_shared::{List, ListType, Tag};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::tag_badge::TagBadge;
use crate::components::tag_selector::TagSelector;

pub fn list_type_label(lt: &ListType) -> &'static str {
    match lt {
        ListType::Checklist => "Checklist",
        ListType::Zakupy => "Zakupy",
        ListType::Pakowanie => "Pakowanie",
        ListType::Terminarz => "Terminarz",
        ListType::Custom => "Custom",
    }
}

pub fn list_type_icon(lt: &ListType) -> &'static str {
    match lt {
        ListType::Checklist => "✅",
        ListType::Zakupy => "🛒",
        ListType::Pakowanie => "🧳",
        ListType::Terminarz => "📅",
        ListType::Custom => "📝",
    }
}

#[component]
pub fn ListCard(
    list: List,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] list_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] on_delete: Option<Callback<String>>,
) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list_type_icon(&list.list_type);
    let label = list_type_label(&list.list_type);

    let navigate = use_navigate();
    let href_clone = href.clone();

    let assigned_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| list_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    let list_id_for_delete = list.id.clone();
    let on_delete_clone = on_delete.clone();

    view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative"
            on:click=move |_| { navigate(&href_clone, Default::default()); }
        >
            // Delete button — positioned absolute, stop_propagation prevents card navigation
            {on_delete_clone.map(|cb| {
                let lid = list_id_for_delete.clone();
                view! {
                    <button
                        type="button"
                        aria-label="Usuń listę"
                        class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-40 hover:opacity-100"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(lid.clone());
                        }
                    >
                        "\u{1F5D1}"
                    </button>
                }
            })}

            <div class="card-body p-4">
                <h3 class="card-title text-base">{list.name.clone()}</h3>
                <span class="text-sm text-base-content/60">{icon} " " {label}</span>
                {if on_tag_toggle.is_some() || !assigned_tags.is_empty() {
                    view! {
                        <div
                            class="tag-list mt-2"
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                        >
                            {assigned_tags.into_iter().map(|t| {
                                let tid = t.id.clone();
                                let cb = on_tag_toggle.clone();
                                if let Some(c) = cb {
                                    let remove_cb = Callback::new(move |_: String| c.run(tid.clone()));
                                    view! { <TagBadge tag=t on_remove=remove_cb /> }.into_any()
                                } else {
                                    view! { <TagBadge tag=t /> }.into_any()
                                }
                            }).collect::<Vec<_>>()}
                            {on_tag_toggle.map(|cb| view! {
                                <TagSelector
                                    all_tags=all_tags.clone()
                                    selected_tag_ids=list_tag_ids.clone()
                                    on_toggle=cb
                                />
                            })}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}
