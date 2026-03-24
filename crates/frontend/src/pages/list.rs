use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::add_item_input::AddItemInput;
use crate::components::add_group_input::AddGroupInput;
use crate::components::date_item_row::{DateItemRow, get_today, is_overdue, is_upcoming, sort_by_due_date};
use crate::components::item_actions::create_item_actions;
use crate::components::item_row::ItemRow;
use crate::components::list_header::ListHeader;
use crate::components::sublist_section::SublistSection;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_selector::TagSelector;
use kartoteka_shared::{Item, ItemTagLink, List, ListTagLink, Tag};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let items = RwSignal::new(Vec::<Item>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let list_tag_links = RwSignal::new(Vec::<ListTagLink>::new());
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal(Option::<String>::None);

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let navigate = use_navigate();
    let list_name = RwSignal::new(String::new());
    let list_has_quantity = RwSignal::new(false);
    let list_has_due_date = RwSignal::new(false);
    let sublists = RwSignal::new(Vec::<List>::new());

    // Initial fetch
    let lid = list_id();
    leptos::task::spawn_local(async move {
        if let Ok(list) = api::fetch_list(&lid).await {
            list_name.set(list.name);
            list_has_quantity.set(list.has_quantity);
            list_has_due_date.set(list.has_due_date);
        }
        match api::fetch_items(&lid).await {
            Ok(fetched) => items.set(fetched),
            Err(e) => set_error.set(Some(e)),
        }
        if let Ok(fetched_tags) = api::fetch_tags().await {
            all_tags.set(fetched_tags);
        }
        if let Ok(links) = api::fetch_item_tag_links().await {
            item_tag_links.set(links);
        }
        if let Ok(fetched_sublists) = api::fetch_sublists(&lid).await {
            sublists.set(fetched_sublists);
        }
        if let Ok(links) = api::fetch_list_tag_links().await {
            let filtered: Vec<ListTagLink> =
                links.into_iter().filter(|l| l.list_id == lid).collect();
            list_tag_links.set(filtered);
        }
        set_loading.set(false);
    });

    let actions = create_item_actions(items, list_id(), Some(set_error));
    let on_add = actions.on_add;
    let on_toggle = actions.on_toggle;
    let on_delete = actions.on_delete;
    let on_description_save = actions.on_description_save;
    let on_quantity_change = actions.on_quantity_change;

    // Item tag toggle callback with optimistic updates
    let on_tag_toggle = Callback::new(move |(item_id, tag_id): (String, String)| {
        let has_tag = item_tag_links
            .read()
            .iter()
            .any(|l| l.item_id == item_id && l.tag_id == tag_id);
        if has_tag {
            item_tag_links.update(|links| {
                links.retain(|l| !(l.item_id == item_id && l.tag_id == tag_id))
            });
            let iid = item_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_item(&iid, &tid).await;
            });
        } else {
            item_tag_links.update(|links| {
                links.push(ItemTagLink {
                    item_id: item_id.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let iid = item_id.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_item(&iid, &tid).await;
            });
        }
    });

    // List tag toggle callback with optimistic updates
    let lid_for_tag = list_id();
    let on_list_tag_toggle = Callback::new(move |tag_id: String| {
        let has_tag = list_tag_links.read().iter().any(|l| l.tag_id == tag_id);
        if has_tag {
            list_tag_links.update(|links| links.retain(|l| l.tag_id != tag_id));
            let lid = lid_for_tag.clone();
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::remove_tag_from_list(&lid, &tid).await;
            });
        } else {
            let lid = lid_for_tag.clone();
            list_tag_links.update(|links| {
                links.push(ListTagLink {
                    list_id: lid.clone(),
                    tag_id: tag_id.clone(),
                })
            });
            let tid = tag_id.clone();
            leptos::task::spawn_local(async move {
                let _ = api::assign_tag_to_list(&lid, &tid).await;
            });
        }
    });

    // Move item callback (main list items)
    let on_move_main = Callback::new(move |(item_id, target_list_id): (String, String)| {
        items.update(|list| list.retain(|i| i.id != item_id));
        leptos::task::spawn_local(async move {
            let _ = api::move_item(&item_id, &target_list_id).await;
        });
    });

    // When an item is moved OUT of a sublist, add it to main list if target is main
    let parent_lid = list_id();
    let on_item_moved_out = Callback::new(move |(moved_item, target_list_id): (Item, String)| {
        if target_list_id == parent_lid {
            items.update(|list| list.push(moved_item));
        }
        // If target is another sublist, that sublist will show it on next refetch
    });

    let on_delete_list = Callback::new(move |_: ()| {
        let lid = list_id();
        let nav = navigate.clone();
        leptos::task::spawn_local(async move {
            match api::delete_list(&lid).await {
                Ok(()) => {
                    toast.push("Lista usunięta".into(), ToastKind::Success);
                    nav("/", Default::default());
                }
                Err(e) => toast.push(e, ToastKind::Error),
            }
        });
    });

    let navigate_archive = use_navigate();
    let on_archive = Callback::new(move |_: ()| {
        let lid = list_id();
        let nav = navigate_archive.clone();
        leptos::task::spawn_local(async move {
            match api::archive_list(&lid).await {
                Ok(_) => {
                    toast.push("Lista zarchiwizowana".into(), ToastKind::Success);
                    nav("/", Default::default());
                }
                Err(e) => toast.push(e, ToastKind::Error),
            }
        });
    });

    let on_reset = Callback::new(move |_: ()| {
        let lid = list_id();
        leptos::task::spawn_local(async move {
            match api::reset_list(&lid).await {
                Ok(()) => {
                    items.update(|list| {
                        for item in list.iter_mut() {
                            item.completed = false;
                            item.actual_quantity = Some(0);
                        }
                    });
                    toast.push("Lista zresetowana".into(), ToastKind::Success);
                }
                Err(e) => toast.push(e, ToastKind::Error),
            }
        });
    });

    let on_create_group = Callback::new(move |name: String| {
        let lid = list_id();
        leptos::task::spawn_local(async move {
            match api::create_sublist(&lid, &name).await {
                Ok(sl) => sublists.update(|list| list.push(sl)),
                Err(_) => {}
            }
        });
    });

    let (active_tag_filter, set_active_tag_filter) = signal(Option::<String>::None);

    let sorted_items = move || {
        let mut list = items.get();
        list.sort_by(|a, b| {
            a.completed
                .cmp(&b.completed)
                .then(a.position.cmp(&b.position))
        });
        list
    };

    let filtered_items = move || {
        let items = sorted_items();
        match active_tag_filter.get() {
            None => items,
            Some(tid) => {
                let tagged_item_ids: Vec<String> = item_tag_links
                    .read()
                    .iter()
                    .filter(|l| l.tag_id == tid)
                    .map(|l| l.item_id.clone())
                    .collect();
                items
                    .into_iter()
                    .filter(|i| tagged_item_ids.contains(&i.id))
                    .collect()
            }
        }
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            {move || view! {
                <ListHeader
                    list_name=list_name.get()
                    list_id=list_id()
                    item_count=items.read().len()
                    on_delete_confirmed=on_delete_list
                    on_archive=on_archive
                    on_reset=on_reset
                />
            }}

            // List tag management
            {move || {
                let links = list_tag_links.read();
                let tags = all_tags.read();
                let assigned_ids: Vec<String> = links.iter().map(|l| l.tag_id.clone()).collect();
                let assigned: Vec<Tag> = tags
                    .iter()
                    .filter(|t| assigned_ids.contains(&t.id))
                    .cloned()
                    .collect();
                view! {
                    <div class="flex flex-wrap items-center gap-1 mb-3">
                        {assigned.into_iter().map(|t| {
                            let tid = t.id.clone();
                            let cb = on_list_tag_toggle.clone();
                            view! {
                                <TagBadge
                                    tag=t
                                    on_remove=Callback::new(move |_: String| cb.run(tid.clone()))
                                />
                            }
                        }).collect::<Vec<_>>()}
                        {if !tags.is_empty() {
                            view! {
                                <TagSelector
                                    all_tags=tags.clone()
                                    selected_tag_ids=assigned_ids
                                    on_toggle=on_list_tag_toggle
                                />
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                }
            }}

            {move || view! { <AddItemInput on_submit=on_add has_quantity=list_has_quantity.get() has_due_date=list_has_due_date.get() /> }}

            // Tag filter bar
            {move || {
                let tags = all_tags.read();
                if tags.is_empty() {
                    view! {}.into_any()
                } else {
                    let tags_list: Vec<Tag> = tags.clone();
                    view! {
                        <div class="flex flex-wrap gap-1 mb-3">
                            <button
                                class=move || if active_tag_filter.get().is_none() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-ghost" }
                                on:click=move |_| set_active_tag_filter.set(None)
                            >"Wszystkie"</button>
                            {tags_list.into_iter().map(|t| {
                                let tid_class = t.id.clone();
                                let tid_style = t.id.clone();
                                let tid_click = t.id.clone();
                                let tname = t.name.clone();
                                let tcolor_class = t.color.clone();
                                let tcolor_style = t.color.clone();
                                view! {
                                    <button
                                        class=move || if active_tag_filter.get().as_deref() == Some(&tid_class) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                                        style=move || format!("border-color: {}; color: {}", tcolor_style, if active_tag_filter.get().as_deref() == Some(&tid_style) { "#fff" } else { &tcolor_class })
                                        on:click=move |_| set_active_tag_filter.set(Some(tid_click.clone()))
                                    >{tname}</button>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }
            }}

            {move || {
                if loading.get() {
                    view! { <p>"Wczytywanie..."</p> }.into_any()
                } else if let Some(e) = error.get() {
                    view! { <p style="color: red;">{format!("Błąd: {e}")}</p> }.into_any()
                } else if items.read().is_empty() && sublists.read().is_empty() {
                    view! { <div class="text-center text-base-content/50 py-12">"Lista jest pusta"</div> }.into_any()
                } else {
                    view! {
                        <div>
                            // Main list items
                            {move || {
                                if list_has_due_date.get() {
                                    // Date view: group into overdue, upcoming, done
                                    let today = get_today();
                                    let all = filtered_items();

                                    let mut overdue: Vec<Item> = all.iter()
                                        .filter(|i| is_overdue(i, &today))
                                        .cloned().collect();
                                    sort_by_due_date(&mut overdue);

                                    let mut upcoming: Vec<Item> = all.iter()
                                        .filter(|i| is_upcoming(i, &today))
                                        .cloned().collect();
                                    sort_by_due_date(&mut upcoming);

                                    let mut done: Vec<Item> = all.iter()
                                        .filter(|i| i.completed)
                                        .cloned().collect();
                                    sort_by_due_date(&mut done);

                                    let tags_for_date = all_tags.get();
                                    let links_for_date = item_tag_links.get();
                                    let render_section = |label: &str, css: &str, items: Vec<Item>, tags: Vec<Tag>, links: Vec<ItemTagLink>| {
                                        let label = label.to_string();
                                        let css = css.to_string();
                                        if items.is_empty() {
                                            view! {}.into_any()
                                        } else {
                                            view! {
                                                <div class="mb-4">
                                                    <h3 class=format!("text-sm font-semibold uppercase tracking-wide mb-2 {}", css)>{label}</h3>
                                                    {items.into_iter().map(|item| {
                                                        let item_id = item.id.clone();
                                                        let item_tags: Vec<String> = links.iter()
                                                            .filter(|l| l.item_id == item.id)
                                                            .map(|l| l.tag_id.clone())
                                                            .collect();
                                                        let tags_clone = tags.clone();
                                                        let tog_cb = on_tag_toggle;
                                                        let item_tag_toggle = Callback::new(move |tag_id: String| {
                                                            tog_cb.run((item_id.clone(), tag_id));
                                                        });
                                                        view! {
                                                            <DateItemRow
                                                                item=item
                                                                on_toggle=on_toggle
                                                                on_delete=on_delete
                                                                all_tags=tags_clone
                                                                item_tag_ids=item_tags
                                                                on_tag_toggle=item_tag_toggle
                                                            />
                                                        }
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            }.into_any()
                                        }
                                    };

                                    view! {
                                        <div>
                                            {render_section("Zaległe", "text-error", overdue, tags_for_date.clone(), links_for_date.clone())}
                                            {render_section("Nadchodzące", "text-warning", upcoming, tags_for_date.clone(), links_for_date.clone())}
                                            {render_section("Zrobione", "text-base-content/40", done, tags_for_date, links_for_date)}
                                        </div>
                                    }.into_any()
                                } else {
                                    // Normal position-sorted view
                                    let main_move_targets: Vec<(String, String)> = sublists.get().iter()
                                        .map(|s| (s.id.clone(), s.name.clone()))
                                        .collect();
                                    view! {
                                        <div>
                                            {filtered_items().iter().map(|item| {
                                                let item_id = item.id.clone();
                                                let item_tags: Vec<String> = item_tag_links.read().iter()
                                                    .filter(|l| l.item_id == item.id)
                                                    .map(|l| l.tag_id.clone())
                                                    .collect();
                                                let tags_clone = all_tags.get();
                                                let tog_cb = on_tag_toggle.clone();
                                                let item_tag_toggle = Callback::new(move |tag_id: String| {
                                                    tog_cb.run((item_id.clone(), tag_id));
                                                });
                                                let mt = main_move_targets.clone();
                                                view! {
                                                    <ItemRow
                                                        item=item.clone()
                                                        on_toggle=on_toggle
                                                        on_delete=on_delete
                                                        all_tags=tags_clone
                                                        item_tag_ids=item_tags
                                                        on_tag_toggle=item_tag_toggle
                                                        on_description_save=on_description_save
                                                        has_quantity=list_has_quantity.get()
                                                        on_quantity_change=on_quantity_change
                                                        move_targets=mt
                                                        on_move=on_move_main
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }}

                            // Sub-lists
                            {move || {
                                let subs = sublists.get();
                                if subs.is_empty() {
                                    view! {}.into_any()
                                } else {
                                    view! {
                                        <div class="mt-6">
                                            {subs.iter().map(|sl| {
                                                let tags = all_tags.get();
                                                let links = item_tag_links.get();
                                                let lid = list_id();
                                                let lname = list_name.get();
                                                let sl_id = sl.id.clone();
                                                let mut mt: Vec<(String, String)> = vec![
                                                    (lid, format!("{lname} (główna)"))
                                                ];
                                                mt.extend(
                                                    subs.iter()
                                                        .filter(|s| s.id != sl_id)
                                                        .map(|s| (s.id.clone(), s.name.clone()))
                                                );
                                                view! {
                                                    <SublistSection
                                                        sublist=sl.clone()
                                                        has_quantity=list_has_quantity.get()
                                                        has_due_date=list_has_due_date.get()
                                                        all_tags=tags
                                                        item_tag_links=links
                                                        on_tag_toggle=on_tag_toggle
                                                        move_targets=mt
                                                        on_item_moved_out=on_item_moved_out
                                                    />
                                                }
                                            }).collect::<Vec<_>>()}
                                        </div>
                                    }.into_any()
                                }
                            }}

                        </div>
                    }.into_any()
                }
            }}

            <AddGroupInput on_submit=on_create_group />
        </div>
    }
}
