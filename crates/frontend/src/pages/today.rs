use std::collections::HashSet;

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::{DateItem, Tag};

use crate::components::common::list_filter_chips::ListFilterChips;
use crate::components::common::loading::LoadingSpinner;
use crate::components::ui::{IconButton, IconButtonVariant};
use crate::server_fns::items::{delete_item, get_today_data, toggle_item};
use crate::server_fns::tags::{get_all_item_tag_links, get_all_tags};
use crate::state::invalidation::{mutate, use_invalidation};
use crate::utils::group_by_list;

#[component]
pub fn TodayPage() -> impl IntoView {
    let invalidation = use_invalidation();
    let show_completed = RwSignal::new(true);
    let hidden_lists: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let today_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Lists]),
        |_| get_today_data(),
    );
    let all_tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags]),
        |_| get_all_tags(),
    );
    let item_tag_links_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Tags]),
        |_| get_all_item_tag_links(),
    );

    let hidden_tags: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let on_toggle = move |item_id: String| {
        mutate(toggle_item(item_id), &[ChangeTopic::Items]);
    };

    let on_delete = move |item_id: String| {
        mutate(delete_item(item_id), &[ChangeTopic::Items]);
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || today_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let all_items: Vec<&DateItem> = data.overdue.iter().chain(data.today.iter()).collect();
                        let total = all_items.len();
                        let completed_count = all_items.iter().filter(|di| di.item.completed).count();

                        let has_overdue = !data.overdue.is_empty();
                        let has_today = !data.today.is_empty();

                        let mut unique_lists: Vec<(String, String)> = Vec::new();
                        let mut seen = HashSet::new();
                        for di in &all_items {
                            if seen.insert(di.item.list_id.clone()) {
                                unique_lists.push((di.item.list_id.clone(), di.list_name.clone()));
                            }
                        }

                        let item_ids: HashSet<String> = all_items.iter().map(|di| di.item.id.clone()).collect();
                        let all_tags: Vec<Tag> = all_tags_res.get().and_then(|r| r.ok()).unwrap_or_default();
                        let links = item_tag_links_res.get().and_then(|r| r.ok()).unwrap_or_default();
                        let relevant_tag_ids: HashSet<String> = links.iter()
                            .filter(|l| item_ids.contains(&l.item_id))
                            .map(|l| l.tag_id.clone())
                            .collect();
                        let relevant_tags: Vec<Tag> = all_tags.into_iter()
                            .filter(|t| relevant_tag_ids.contains(&t.id))
                            .collect();
                        let links = StoredValue::new(links);

                        let overdue_items = data.overdue;
                        let today_items = data.today;

                        view! {
                            <div>
                                <div class="flex items-center justify-between mb-4">
                                    <h1 class="text-2xl font-bold">"Dziś"</h1>
                                    <span class="text-base-content/50 text-sm">{data.today_date}</span>
                                </div>

                                {if !has_overdue && !has_today {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Brak zadań na dziś."
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <div class="flex items-center justify-between mb-3">
                                                <span class="text-sm text-base-content/60" data-testid="today-completion-count">
                                                    {completed_count} "/" {total} " ukończone"
                                                </span>
                                                <label class="flex items-center gap-2 cursor-pointer select-none">
                                                    <span class="text-xs text-base-content/50">"Ukryj ukończone"</span>
                                                    <input
                                                        type="checkbox"
                                                        class="toggle toggle-xs"
                                                        data-testid="hide-completed-toggle"
                                                        prop:checked=move || !show_completed.get()
                                                        on:change=move |ev| show_completed.set(!event_target_checked(&ev))
                                                    />
                                                </label>
                                            </div>

                                            <ListFilterChips
                                                lists=unique_lists
                                                hidden_lists=hidden_lists
                                                tags=relevant_tags
                                                hidden_tags=hidden_tags
                                                show_completed=show_completed
                                            />

                                            {move || {
                                                let hl = hidden_lists.get();
                                                let sc = show_completed.get();
                                                let ht = hidden_tags.get();

                                                let filter = |items: &Vec<DateItem>| -> Vec<DateItem> {
                                                    items.iter()
                                                        .filter(|di| !hl.contains(&di.item.list_id))
                                                        .filter(|di| sc || !di.item.completed)
                                                        .filter(|di| {
                                                            if ht.is_empty() { return true; }
                                                            !links.get_value().iter()
                                                                .filter(|l| l.item_id == di.item.id)
                                                                .any(|l| ht.contains(&l.tag_id))
                                                        })
                                                        .cloned()
                                                        .collect()
                                                };

                                                let visible_overdue = filter(&overdue_items);
                                                let visible_today = filter(&today_items);
                                                let all_hidden = visible_overdue.is_empty() && visible_today.is_empty();
                                                let has_visible_overdue = !visible_overdue.is_empty();

                                                if all_hidden {
                                                    view! {
                                                        <div class="text-center text-base-content/50 py-4">
                                                            "Wszystkie elementy ukończone lub ukryte."
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            {if has_visible_overdue {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                                                            "Zaległe"
                                                                        </h3>
                                                                        {render_groups(group_by_list(visible_overdue), on_toggle, on_delete)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}

                                                            {if !visible_today.is_empty() {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        {if has_overdue && has_visible_overdue {
                                                                            view! {
                                                                                <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                                                                    "Dziś"
                                                                                </h3>
                                                                            }.into_any()
                                                                        } else { ().into_any() }}
                                                                        {render_groups(group_by_list(visible_today), on_toggle, on_delete)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_groups(
    groups: Vec<(String, String, Vec<DateItem>)>,
    on_toggle: impl Fn(String) + Copy + Send + Sync + 'static,
    on_delete: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    groups
        .into_iter()
        .map(|(list_id, list_name, items)| {
            view! {
                <div class="mb-4">
                    <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                            {list_name}
                        </A>
                    </h4>
                    <div class="flex flex-col gap-1">
                        {items.into_iter().map(|date_item| {
                            let item_id = date_item.item.id.clone();
                            let item_id_del = item_id.clone();
                            let href = format!("/lists/{}/items/{}", date_item.item.list_id, date_item.item.id);
                            let completed = date_item.item.completed;
                            let title = date_item.item.title.clone();
                            let deadline = date_item.item.deadline
                                .as_ref()
                                .map(|d| d.to_string());

                            view! {
                                <div class="flex items-center gap-3 p-2 bg-base-200 rounded-lg group">
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-sm checkbox-primary"
                                        checked=completed
                                        on:change=move |_| on_toggle(item_id.clone())
                                    />
                                    <A
                                        href=href
                                        attr:class=move || if completed {
                                            "flex-1 text-sm text-base-content/50 line-through"
                                        } else {
                                            "flex-1 text-sm text-base-content hover:text-primary"
                                        }
                                    >
                                        {title}
                                    </A>
                                    {deadline.map(|d| view! {
                                        <span class="text-xs text-error/70">{d}</span>
                                    })}
                                    <IconButton
                                        icon="x"
                                        label=move_tr!("common-delete")
                                        variant=IconButtonVariant::Danger
                                        class="opacity-0 group-hover:opacity-100"
                                        on_click=Callback::new(move |_| on_delete(item_id_del.clone()))
                                    />
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })
        .collect_view()
}
