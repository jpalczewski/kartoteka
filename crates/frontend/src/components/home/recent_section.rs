use std::collections::HashSet;

use kartoteka_shared::types::{Container, List, ListTagLink, Tag};
use leptos::prelude::*;

use crate::components::lists::{container_card::ContainerCard, list_card::ListCard};

#[component]
pub fn RecentSection(
    recent_lists: Vec<List>,
    recent_containers: Vec<Container>,
    all_tags: Vec<Tag>,
    all_links: Vec<ListTagLink>,
    matching_list_ids: Signal<Option<HashSet<String>>>,
    matching_container_ids: Signal<Option<HashSet<String>>>,
    on_tag_toggle: Callback<(String, String)>,
    on_delete_list: Callback<String>,
) -> impl IntoView {
    let list_filter = matching_list_ids.get_untracked();
    let container_filter = matching_container_ids.get_untracked();
    let filtered_lists: Vec<List> = recent_lists
        .into_iter()
        .filter(|l| list_filter.as_ref().is_none_or(|ids| ids.contains(&l.id)))
        .collect();
    let filtered_containers: Vec<Container> = recent_containers
        .into_iter()
        .filter(|c| {
            container_filter
                .as_ref()
                .is_none_or(|ids| ids.contains(&c.id))
        })
        .collect();

    if filtered_lists.is_empty() && filtered_containers.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <div class="collapse collapse-arrow bg-base-200 mb-4">
            <input type="checkbox" checked />
            <div class="collapse-title font-semibold">
                "🕐 Ostatnio otwarte"
            </div>
            <div class="collapse-content">
                <div class="flex flex-col gap-3 pt-2">
                    {filtered_containers.into_iter().map(|c| {
                        view! { <ContainerCard container=c /> }
                    }).collect::<Vec<_>>()}
                    {filtered_lists.into_iter().map(|list| {
                        let list_id = list.id.clone();
                        let tag_ids: Vec<String> = all_links.iter()
                            .filter(|l| l.list_id == list.id)
                            .map(|l| l.tag_id.clone())
                            .collect();
                        let tog = on_tag_toggle.clone();
                        let del = on_delete_list.clone();
                        let lid = list.id.clone();
                        view! {
                            <ListCard
                                list=list
                                all_tags=all_tags.clone()
                                list_tag_ids=tag_ids
                                on_tag_toggle=Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)))
                                on_delete=Callback::new(move |_: String| del.run(lid.clone()))
                            />
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }.into_any()
}
