use kartoteka_shared::types::{List, Tag};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::tags::tag_list::TagList;

pub fn list_type_icon(lt: &str) -> &'static str {
    match lt {
        "checklist" => "✅",
        "shopping" => "🛒",
        "habits" => "🔄",
        "log" => "📋",
        _ => "📝",
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
    let icon = list_type_icon(&list.list_type.clone());
    let navigate = use_navigate();
    let href_nav = href.clone();

    let list_id_del = list.id.clone();
    let list_name = list.name.clone();

    view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative"
            data-testid="list-card"
            on:click=move |_| { navigate(&href_nav, Default::default()); }
        >
            {on_delete.map(|cb| {
                let lid = list_id_del.clone();
                view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-circle absolute top-2 right-2 z-10 text-error"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(lid.clone());
                        }
                    >
                        {"✕"}
                    </button>
                }
            })}
            <div class="card-body p-4">
                <div class="flex items-center gap-2">
                    <span>{icon}</span>
                    <span class="card-title text-base" data-testid="list-card-title">{list_name}</span>
                </div>
                {match on_tag_toggle {
                    Some(cb) => view! {
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=list_tag_ids.clone()
                            on_toggle=cb
                        />
                    }.into_any(),
                    None => view! {
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=list_tag_ids.clone()
                        />
                    }.into_any(),
                }}
            </div>
        </div>
    }
}
