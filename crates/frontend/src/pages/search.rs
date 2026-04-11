use std::collections::BTreeSet;

use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_query_map};

use crate::api;
use crate::api::client::GlooClient;
use crate::components::common::error_display::ErrorDisplay;
use crate::components::common::loading::LoadingSpinner;
use crate::state::search_route::{
    CompletionFilter, SearchMode, SearchRouteState, search_href, search_state_from_query_map,
};
use kartoteka_shared::*;

#[derive(Clone)]
enum SearchResults {
    Global(Vec<SearchEntityResult>),
    Items(Vec<SearchItemResult>),
}

#[component]
pub fn SearchPage() -> impl IntoView {
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let navigate = use_navigate();
    let query_map = use_query_map();

    let route_state = Memo::new(move |_| {
        let query = query_map.read();
        search_state_from_query_map(&query)
    });

    let form_query = RwSignal::new(String::new());
    let search_title = RwSignal::new(true);
    let search_description = RwSignal::new(true);
    let selected_tags = RwSignal::new(BTreeSet::<String>::new());
    let completion = RwSignal::new(CompletionFilter::All);
    let include_archived = RwSignal::new(false);

    Effect::new(move |_| {
        let state = route_state.get();
        form_query.set(state.query.unwrap_or_default());
        search_title.set(state.search_title);
        search_description.set(state.search_description);
        selected_tags.set(state.tag_ids);
        completion.set(state.completed);
        include_archived.set(state.include_archived);
    });

    let tags_resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let client = client.clone();
            async move { api::fetch_tags(&client).await }
        })
    };

    let search_results = RwSignal::new(SearchResults::Items(Vec::new()));
    let next_cursor = RwSignal::new(None::<String>);
    let search_loading = RwSignal::new(false);
    let search_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let state = route_state.get();
        search_results.set(match state.mode() {
            SearchMode::Global => SearchResults::Global(Vec::new()),
            SearchMode::Items => SearchResults::Items(Vec::new()),
        });
        next_cursor.set(None);
        search_error.set(None);

        if !state.has_search() {
            search_loading.set(false);
            return;
        }

        search_loading.set(true);
        let client = client.clone();
        leptos::task::spawn_local(async move {
            match state.mode() {
                SearchMode::Global => {
                    let query = state.query.clone().unwrap_or_default();
                    match api::search_entities_page(&client, &query).await {
                        Ok(page) => {
                            search_results.set(SearchResults::Global(page.items));
                            next_cursor.set(page.next_cursor);
                            search_error.set(None);
                        }
                        Err(error) => {
                            search_results.set(SearchResults::Global(Vec::new()));
                            next_cursor.set(None);
                            search_error.set(Some(error.to_string()));
                        }
                    }
                }
                SearchMode::Items => match api::search_items_page(&client, &state).await {
                    Ok(page) => {
                        search_results.set(SearchResults::Items(page.items));
                        next_cursor.set(page.next_cursor);
                        search_error.set(None);
                    }
                    Err(error) => {
                        search_results.set(SearchResults::Items(Vec::new()));
                        next_cursor.set(None);
                        search_error.set(Some(error.to_string()));
                    }
                },
            }
            search_loading.set(false);
        });
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let query = form_query.get_untracked().trim().to_string();
        let state = SearchRouteState {
            query: if query.is_empty() { None } else { Some(query) },
            search_title: search_title.get_untracked(),
            search_description: search_description.get_untracked(),
            tag_ids: selected_tags.get_untracked(),
            completed: completion.get_untracked(),
            include_archived: include_archived.get_untracked(),
        };
        navigate(&search_href(&state), Default::default());
    };

    let on_load_more = move |_| {
        let Some(cursor) = next_cursor.get_untracked() else {
            return;
        };
        let mode = route_state.get_untracked().mode();
        search_loading.set(true);
        let client = client.clone();
        leptos::task::spawn_local(async move {
            match mode {
                SearchMode::Global => {
                    match api::fetch_next_page::<SearchEntityResult>(&client, &cursor).await {
                        Ok(page) => {
                            search_results.update(|results| {
                                if let SearchResults::Global(items) = results {
                                    items.extend(page.items);
                                }
                            });
                            next_cursor.set(page.next_cursor);
                            search_error.set(None);
                        }
                        Err(error) => {
                            search_error.set(Some(error.to_string()));
                        }
                    }
                }
                SearchMode::Items => {
                    match api::fetch_next_page::<SearchItemResult>(&client, &cursor).await {
                        Ok(page) => {
                            search_results.update(|results| {
                                if let SearchResults::Items(items) = results {
                                    items.extend(page.items);
                                }
                            });
                            next_cursor.set(page.next_cursor);
                            search_error.set(None);
                        }
                        Err(error) => {
                            search_error.set(Some(error.to_string()));
                        }
                    }
                }
            }
            search_loading.set(false);
        });
    };

    let global_mode_preview = Memo::new(move |_| {
        !form_query.get().trim().is_empty()
            && selected_tags.get().is_empty()
            && completion.get() == CompletionFilter::All
            && !include_archived.get()
    });

    view! {
        <div class="container mx-auto max-w-5xl p-4">
            <div class="mb-6 flex flex-col gap-2">
                <h1 class="text-2xl font-bold">{move_tr!("search-title")}</h1>
                <p class="text-base-content/60">{move_tr!("search-subtitle")}</p>
            </div>

            <form class="card bg-base-200 border border-base-300 mb-6" on:submit=on_submit>
                <div class="card-body gap-4">
                    <div class="flex flex-col gap-2">
                        <label class="text-sm font-medium" for="search-query">{move_tr!("search-query-label")}</label>
                        <input
                            id="search-query"
                            class="input input-bordered w-full"
                            type="search"
                            prop:value=move || form_query.get()
                            on:input=move |ev| form_query.set(event_target_value(&ev))
                            placeholder=move_tr!("search-query-placeholder")
                        />
                    </div>

                    <div class="flex flex-wrap gap-4">
                        <label class="label cursor-pointer gap-2">
                            <input
                                class="checkbox checkbox-sm checkbox-primary"
                                type="checkbox"
                                disabled=move || global_mode_preview.get()
                                prop:checked=move || search_title.get()
                                on:change=move |_| search_title.update(|value| *value = !*value)
                            />
                            <span class="label-text">{move_tr!("search-field-title")}</span>
                        </label>
                        <label class="label cursor-pointer gap-2">
                            <input
                                class="checkbox checkbox-sm checkbox-primary"
                                type="checkbox"
                                disabled=move || global_mode_preview.get()
                                prop:checked=move || search_description.get()
                                on:change=move |_| search_description.update(|value| *value = !*value)
                            />
                            <span class="label-text">{move_tr!("search-field-description")}</span>
                        </label>
                        <label class="label cursor-pointer gap-2">
                            <input
                                class="checkbox checkbox-sm checkbox-primary"
                                type="checkbox"
                                prop:checked=move || include_archived.get()
                                on:change=move |_| include_archived.update(|value| *value = !*value)
                            />
                            <span class="label-text">{move_tr!("search-include-archived")}</span>
                        </label>
                    </div>
                    {move || if global_mode_preview.get() {
                        view! {
                            <p class="text-sm text-base-content/60">{move_tr!("search-global-mode-hint")}</p>
                        }.into_any()
                    } else {
                        ().into_any()
                    }}

                    <div class="flex flex-col gap-2">
                        <label class="text-sm font-medium" for="search-completed">{move_tr!("search-completed-label")}</label>
                        <select
                            id="search-completed"
                            class="select select-bordered w-full max-w-xs"
                            on:change=move |ev| {
                                let value = event_target_value(&ev);
                                completion.set(match value.as_str() {
                                    "open" => CompletionFilter::Open,
                                    "done" => CompletionFilter::Done,
                                    _ => CompletionFilter::All,
                                });
                            }
                        >
                            <option value="all" selected=move || completion.get() == CompletionFilter::All>{move_tr!("search-completed-all")}</option>
                            <option value="open" selected=move || completion.get() == CompletionFilter::Open>{move_tr!("search-completed-open")}</option>
                            <option value="done" selected=move || completion.get() == CompletionFilter::Done>{move_tr!("search-completed-done")}</option>
                        </select>
                    </div>

                    <div class="flex flex-col gap-2">
                        <span class="text-sm font-medium">{move_tr!("search-tags-label")}</span>
                        <Suspense fallback=|| view! { <LoadingSpinner/> }>
                            {move || {
                                match tags_resource.get() {
                                    Some(Ok(tags)) if !tags.is_empty() => view! {
                                        <div class="flex flex-wrap gap-2">
                                            {tags.into_iter().map(|tag| {
                                                let tag_id = tag.id.clone();
                                                let checked_id = tag.id.clone();
                                                let color = tag.color.clone();
                                                view! {
                                                    <label
                                                        class="badge badge-lg gap-2 cursor-pointer border-0"
                                                        style=format!("background-color: {}; color: white;", color)
                                                    >
                                                        <input
                                                            class="checkbox checkbox-xs checkbox-primary"
                                                            type="checkbox"
                                                            prop:checked=move || selected_tags.get().contains(&checked_id)
                                                            on:change=move |_| {
                                                                selected_tags.update(|ids| {
                                                                    if !ids.remove(&tag_id) {
                                                                        ids.insert(tag_id.clone());
                                                                    }
                                                                });
                                                            }
                                                        />
                                                        {tag.name}
                                                    </label>
                                                }
                                            }).collect_view()}
                                        </div>
                                    }.into_any(),
                                    Some(Ok(_)) => view! {
                                        <p class="text-sm text-base-content/50">{move_tr!("search-tags-empty")}</p>
                                    }.into_any(),
                                    Some(Err(error)) => view! {
                                        <ErrorDisplay message=error.to_string() />
                                    }.into_any(),
                                    None => ().into_any(),
                                }
                            }}
                        </Suspense>
                    </div>

                    <div class="flex justify-end">
                        <button class="btn btn-primary" type="submit">{move_tr!("search-submit")}</button>
                    </div>
                </div>
            </form>

            {move || {
                let state = route_state.get();
                if !state.has_search() {
                    return view! {
                        <div class="card bg-base-200 border border-base-300">
                            <div class="card-body text-base-content/60">
                                {move_tr!("search-empty-state")}
                            </div>
                        </div>
                    }.into_any();
                }

                if let Some(error) = search_error.get() {
                    return view! {
                        <ErrorDisplay message=error />
                    }.into_any();
                }

                if search_loading.get() && matches!(search_results.get(), SearchResults::Global(ref items) if items.is_empty()) {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                if search_loading.get() && matches!(search_results.get(), SearchResults::Items(ref items) if items.is_empty()) {
                    return view! { <LoadingSpinner/> }.into_any();
                }

                match search_results.get() {
                    SearchResults::Global(results) => {
                        if results.is_empty() {
                            return view! {
                                <div class="card bg-base-200 border border-base-300">
                                    <div class="card-body text-base-content/60">
                                        {move_tr!("search-no-results")}
                                    </div>
                                </div>
                            }.into_any();
                        }

                        view! {
                            <div class="flex flex-col gap-3">
                                {results.into_iter().map(render_global_search_result).collect_view()}
                                {next_cursor.get().map(|_| view! {
                                    <div class="flex justify-center pt-2">
                                        <button
                                            class="btn btn-outline"
                                            type="button"
                                            on:click=on_load_more
                                            disabled=move || search_loading.get()
                                        >
                                            {move || if search_loading.get() {
                                                move_tr!("common-loading")
                                            } else {
                                                move_tr!("search-load-more")
                                            }}
                                        </button>
                                    </div>
                                })}
                            </div>
                        }.into_any()
                    }
                    SearchResults::Items(items) => {
                        if items.is_empty() {
                            return view! {
                                <div class="card bg-base-200 border border-base-300">
                                    <div class="card-body text-base-content/60">
                                        {move_tr!("search-no-results")}
                                    </div>
                                </div>
                            }.into_any();
                        }

                        view! {
                            <div class="flex flex-col gap-3">
                                {items.into_iter().map(render_item_search_result).collect_view()}
                                {next_cursor.get().map(|_| view! {
                                    <div class="flex justify-center pt-2">
                                        <button
                                            class="btn btn-outline"
                                            type="button"
                                            on:click=on_load_more
                                            disabled=move || search_loading.get()
                                        >
                                            {move || if search_loading.get() {
                                                move_tr!("common-loading")
                                            } else {
                                                move_tr!("search-load-more")
                                            }}
                                        </button>
                                    </div>
                                })}
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

fn render_item_search_result(item: SearchItemResult) -> impl IntoView {
    view! {
        <article class="card bg-base-200 border border-base-300">
            <div class="card-body gap-3">
                <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                    <div class="flex flex-col gap-1">
                        <A href=format!("/lists/{}/items/{}", item.list_id, item.id) attr:class="text-lg font-semibold link link-hover text-primary">
                            {item.title.clone()}
                        </A>
                        <A href=format!("/lists/{}", item.list_id) attr:class="text-sm text-base-content/60 link link-hover">
                            {item.list_name.clone()}
                        </A>
                    </div>
                    <div class="flex flex-wrap gap-2">
                        <span class=if item.completed { "badge badge-success" } else { "badge badge-ghost" }>
                            {if item.completed {
                                move_tr!("search-status-done")
                            } else {
                                move_tr!("search-status-open")
                            }}
                        </span>
                        {item.list_archived.then(|| view! {
                            <span class="badge badge-warning">{move_tr!("search-status-archived")}</span>
                        })}
                    </div>
                </div>

                {item.description.as_ref().map(|description| view! {
                    <p class="text-sm text-base-content/70 whitespace-pre-wrap">{description.clone()}</p>
                })}
            </div>
        </article>
    }
}

fn render_global_search_result(result: SearchEntityResult) -> impl IntoView {
    match result.entity_type {
        SearchEntityType::Item => {
            let list_id = result.list_id.unwrap_or_default();
            let list_name = result.list_name.unwrap_or_default();
            let completed = result.completed.unwrap_or(false);
            let archived = result.archived.unwrap_or(false);
            view! {
                <article class="card bg-base-200 border border-base-300">
                    <div class="card-body gap-3">
                        <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                            <div class="flex flex-col gap-1">
                                <div class="flex flex-wrap gap-2 items-center">
                                    <span class="badge badge-outline">{move_tr!("search-entity-item")}</span>
                                    <A href=format!("/lists/{}/items/{}", list_id, result.id) attr:class="text-lg font-semibold link link-hover text-primary">
                                        {result.name}
                                    </A>
                                </div>
                                <A href=format!("/lists/{}", list_id) attr:class="text-sm text-base-content/60 link link-hover">
                                    {list_name}
                                </A>
                            </div>
                            <div class="flex flex-wrap gap-2">
                                <span class=if completed { "badge badge-success" } else { "badge badge-ghost" }>
                                    {if completed { move_tr!("search-status-done") } else { move_tr!("search-status-open") }}
                                </span>
                                {archived.then(|| view! {
                                    <span class="badge badge-warning">{move_tr!("search-status-archived")}</span>
                                })}
                            </div>
                        </div>
                        {result.description.map(|description| view! {
                            <p class="text-sm text-base-content/70 whitespace-pre-wrap">{description}</p>
                        })}
                    </div>
                </article>
            }.into_any()
        }
        SearchEntityType::List => {
            let archived = result.archived.unwrap_or(false);
            let type_label = match result.list_type.unwrap_or(ListType::Custom) {
                ListType::Checklist => move_tr!("lists-type-checklist"),
                ListType::Zakupy => move_tr!("lists-type-shopping"),
                ListType::Pakowanie => move_tr!("lists-type-packing"),
                ListType::Terminarz => move_tr!("lists-type-schedule"),
                ListType::Custom => move_tr!("lists-type-custom"),
            };
            view! {
                <article class="card bg-base-200 border border-base-300">
                    <div class="card-body gap-3">
                        <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                            <div class="flex flex-col gap-1">
                                <div class="flex flex-wrap gap-2 items-center">
                                    <span class="badge badge-outline">{move_tr!("search-entity-list")}</span>
                                    <A href=format!("/lists/{}", result.id) attr:class="text-lg font-semibold link link-hover text-primary">
                                        {result.name}
                                    </A>
                                </div>
                                <span class="text-sm text-base-content/60">{type_label}</span>
                            </div>
                            <div class="flex flex-wrap gap-2">
                                {archived.then(|| view! {
                                    <span class="badge badge-warning">{move_tr!("search-status-archived")}</span>
                                })}
                            </div>
                        </div>
                        {result.description.map(|description| view! {
                            <p class="text-sm text-base-content/70 whitespace-pre-wrap">{description}</p>
                        })}
                    </div>
                </article>
            }.into_any()
        }
        SearchEntityType::Container => {
            let status_label = result.status.map(|status| match status {
                ContainerStatus::Active => move_tr!("lists-container-status-active"),
                ContainerStatus::Done => move_tr!("lists-container-status-done"),
                ContainerStatus::Paused => move_tr!("lists-container-status-paused"),
            });
            view! {
                <article class="card bg-base-200 border border-base-300">
                    <div class="card-body gap-3">
                        <div class="flex flex-col gap-2 md:flex-row md:items-start md:justify-between">
                            <div class="flex flex-col gap-1">
                                <div class="flex flex-wrap gap-2 items-center">
                                    <span class="badge badge-outline">{move_tr!("search-entity-container")}</span>
                                    <A href=format!("/containers/{}", result.id) attr:class="text-lg font-semibold link link-hover text-primary">
                                        {result.name}
                                    </A>
                                </div>
                            </div>
                            <div class="flex flex-wrap gap-2">
                                {status_label.map(|label| view! {
                                    <span class="badge badge-ghost">{label}</span>
                                })}
                            </div>
                        </div>
                        {result.description.map(|description| view! {
                            <p class="text-sm text-base-content/70 whitespace-pre-wrap">{description}</p>
                        })}
                    </div>
                </article>
            }.into_any()
        }
    }
}
