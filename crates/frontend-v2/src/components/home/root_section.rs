use kartoteka_shared::types::{Container, List, ListTagLink, Tag};
use leptos::prelude::*;

use crate::components::lists::{container_card::ContainerCard, list_card::ListCard};

#[component]
pub fn RootSection(
    root_containers: Vec<Container>,
    root_lists: Vec<List>,
    all_tags: Vec<Tag>,
    all_links: Vec<ListTagLink>,
    active_tag_filter: ReadSignal<Option<String>>,
    on_tag_toggle: Callback<(String, String)>,
    on_delete_list: Callback<String>,
    on_delete_container: Callback<String>,
    on_pin_container: Callback<String>,
) -> impl IntoView {
    let filter = active_tag_filter.get();
    let filtered_lists: Vec<List> = root_lists.into_iter().filter(|l| match &filter {
        None => true,
        Some(tid) => all_links.iter().any(|lnk| lnk.list_id == l.id && &lnk.tag_id == tid),
    }).collect();

    let has_containers = !root_containers.is_empty();
    let total_visible = root_containers.len() + filtered_lists.len();

    view! {
        <div>
            {has_containers.then(|| view! {
                <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                    "Foldery i projekty"
                </h3>
            })}
            <div class="flex flex-col gap-3 mb-4">
                {root_containers.into_iter().map(|c| {
                    let cid_del = c.id.clone();
                    let cid_pin = c.id.clone();
                    let del = on_delete_container.clone();
                    let pin = on_pin_container.clone();
                    view! {
                        <ContainerCard
                            container=c
                            on_delete=Callback::new(move |_: String| del.run(cid_del.clone()))
                            on_pin=Callback::new(move |_: String| pin.run(cid_pin.clone()))
                        />
                    }
                }).collect::<Vec<_>>()}
            </div>

            {(has_containers && !filtered_lists.is_empty()).then(|| view! {
                <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                    "Listy"
                </h3>
            })}
            <div class="flex flex-col gap-3">
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

            {(total_visible == 0).then(|| view! {
                <div class="text-center text-base-content/50 py-12">
                    "Brak list. Utwórz pierwszą listę powyżej."
                </div>
            })}
        </div>
    }
}
