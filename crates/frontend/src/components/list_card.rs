use kartoteka_shared::{List, ListType, Tag};
use leptos::prelude::*;

use crate::components::tag_badge::TagBadge;

fn list_type_label(lt: &ListType) -> &'static str {
    match lt {
        ListType::Shopping => "Zakupy",
        ListType::Packing => "Pakowanie",
        ListType::Project => "Projekt",
        ListType::Custom => "Lista",
    }
}

fn list_type_icon(lt: &ListType) -> &'static str {
    match lt {
        ListType::Shopping => "\u{1F6D2}",
        ListType::Packing => "\u{1F9F3}",
        ListType::Project => "\u{1F4CB}",
        ListType::Custom => "\u{1F4DD}",
    }
}

#[component]
pub fn ListCard(list: List, #[prop(default = vec![])] tags: Vec<Tag>) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    let icon = list_type_icon(&list.list_type);
    let label = list_type_label(&list.list_type);

    view! {
        <a href=href style="text-decoration: none; color: inherit;">
            <div class="card">
                <h3>{list.name.clone()}</h3>
                <span class="meta">{icon} " " {label}</span>
                {if !tags.is_empty() {
                    view! {
                        <div class="tag-list">
                            {tags.into_iter().map(|t| view! { <TagBadge tag=t/> }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
        </a>
    }
}
