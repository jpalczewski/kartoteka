use kartoteka_shared::types::{Container, List, ListTagLink, Tag};
use leptos::prelude::*;

use crate::components::lists::{container_card::ContainerCard, list_card::ListCard};

#[component]
pub fn PinnedSection(
    pinned_lists: Vec<List>,
    pinned_containers: Vec<Container>,
    all_tags: Vec<Tag>,
    all_links: Vec<ListTagLink>,
    _active_tag_filter: ReadSignal<Option<String>>,
    on_tag_toggle: Callback<(String, String)>,
    on_delete_list: Callback<String>,
    on_pin_container: Callback<String>,
) -> impl IntoView {
    if pinned_lists.is_empty() && pinned_containers.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <div class="collapse collapse-arrow bg-base-200 mb-4">
            <input type="checkbox" checked />
            <div class="collapse-title font-semibold">
                "📌 Przypięte"
            </div>
            <div class="collapse-content">
                <div class="flex flex-col gap-3 pt-2">
                    {pinned_containers.into_iter().map(|c| {
                        let cid = c.id.clone();
                        let pin_cb = on_pin_container.clone();
                        view! {
                            <ContainerCard
                                container=c
                                on_pin=Callback::new(move |_: String| pin_cb.run(cid.clone()))
                            />
                        }
                    }).collect::<Vec<_>>()}
                    {pinned_lists.into_iter().map(|list| {
                        let list_id = list.id.clone();
                        let tag_ids: Vec<String> = all_links.iter()
                            .filter(|l| l.list_id == list.id)
                            .map(|l| l.tag_id.clone())
                            .collect();
                        let tog = on_tag_toggle.clone();
                        let del = on_delete_list.clone();
                        let lid_del = list.id.clone();
                        view! {
                            <ListCard
                                list=list
                                all_tags=all_tags.clone()
                                list_tag_ids=tag_ids
                                on_tag_toggle=Callback::new(move |tag_id: String| tog.run((list_id.clone(), tag_id)))
                                on_delete=Callback::new(move |_: String| del.run(lid_del.clone()))
                            />
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </div>
        </div>
    }.into_any()
}
