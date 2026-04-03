use kartoteka_shared::{List, ListType, Tag};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

use crate::components::tag_list::TagList;

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
    let type_label = match &list.list_type {
        ListType::Checklist => move_tr!("lists-type-checklist"),
        ListType::Zakupy => move_tr!("lists-type-shopping"),
        ListType::Pakowanie => move_tr!("lists-type-packing"),
        ListType::Terminarz => move_tr!("lists-type-schedule"),
        ListType::Custom => move_tr!("lists-type-custom"),
    };

    let navigate = use_navigate();
    let href_clone = href.clone();

    let has_tags = !list_tag_ids.is_empty() || on_tag_toggle.is_some();

    let list_id_for_delete = list.id.clone();
    let on_delete_clone = on_delete.clone();

    view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative transition-all duration-200 ease-out hover:-translate-y-0.5 hover:border-primary/25 hover:shadow-lg"
            on:click=move |_| { navigate(&href_clone, Default::default()); }
        >
            // Delete button — positioned absolute, stop_propagation prevents card navigation
            {on_delete_clone.map(|cb| {
                let lid = list_id_for_delete.clone();
                view! {
                    <button
                        type="button"
                        aria-label={move_tr!("lists-delete-list-aria")}
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
                <span class="text-sm text-base-content/60">{icon} " " {type_label}</span>
                {if has_tags {
                    view! {
                        <div
                            class="tag-list mt-2 overflow-visible"
                            on:click=|ev: web_sys::MouseEvent| ev.stop_propagation()
                        >
                            <TagList
                                all_tags=all_tags.clone()
                                selected_tag_ids=list_tag_ids.clone()
                                on_toggle=on_tag_toggle
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </div>
    }
}
