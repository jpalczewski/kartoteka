pub mod detail;

use std::collections::HashSet;

use kartoteka_shared::{ChangeTopic, DEFAULT_TAG_COLOR, TagUsage};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::components::tags::color_picker::TagColorPicker;
use crate::components::tags::tag_tree::{
    TagTreeRow, build_tag_tree, filter_tags, get_descendant_ids,
};
use crate::components::ui::{Icon, IconButton};
use crate::server_fns::tags::{
    create_tag, delete_tag, get_all_tags, get_tag_usage_counts, update_tag,
};
use crate::state::invalidation::{mutate, use_invalidation};

/// A delete waiting for confirmation, with the numbers the dialog mentions.
#[derive(Clone)]
struct PendingDelete {
    id: String,
    name: String,
    subtags: usize,
    usage: TagUsage,
}

/// Tags page: a collapsible, searchable tag tree with inline create, rename and
/// delete. The tree, its usage counts and the delete dialog all read the same
/// resource, so one `Suspense` covers the whole page.
#[component]
pub fn TagsPage() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let invalidation = use_invalidation();

    // Tag list and usage counts load together: the SSR pass then has a single
    // suspense wait, so the server-rendered rows and the hydrated DOM agree.
    let tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists]),
        |_| async move {
            let tags = get_all_tags().await?;
            let usage = get_tag_usage_counts().await.unwrap_or_default();
            Ok::<_, ServerFnError>((tags, usage))
        },
    );

    let (new_name, set_new_name) = signal(String::new());
    let (new_color, set_new_color) = signal(DEFAULT_TAG_COLOR.to_string());
    let (query, set_query) = signal(String::new());
    let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let pending_delete: RwSignal<Option<PendingDelete>> = RwSignal::new(None);

    let create_with_parent = move |name: String, parent_tag_id: Option<String>| {
        let color = new_color.get_untracked();
        mutate(
            create_tag(name, None, Some(color), parent_tag_id),
            &[ChangeTopic::Tags],
        );
    };

    let on_create_root = Callback::new(move |_: ()| {
        let name = new_name.get().trim().to_string();
        if name.is_empty() {
            return;
        }
        create_with_parent(name, None);
        set_new_name.set(String::new());
    });

    let on_create_child = Callback::new(move |(parent_id, name): (String, String)| {
        create_with_parent(name, Some(parent_id));
    });

    let on_rename = Callback::new(move |(id, name): (String, String)| {
        if name.trim().is_empty() {
            return;
        }
        mutate(
            update_tag(id, Some(name), None, None, false),
            &[ChangeTopic::Tags],
        );
    });

    let on_delete_confirmed = Callback::new(move |_: ()| {
        let Some(pending) = pending_delete.get_untracked() else {
            return;
        };
        pending_delete.set(None);
        mutate(
            delete_tag(pending.id),
            &[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists],
        );
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="mb-4 flex items-center gap-2">
                <Icon name="tag" size=20 class="text-base-content/60" />
                <h2 class="text-2xl font-bold" data-testid="tags-title">
                    {move_tr!("tags-title")}
                </h2>
            </div>

            // Create row: palette colour, name, add.
            <div class="mb-3 flex flex-wrap items-center gap-2">
                <TagColorPicker
                    value=Signal::derive(move || new_color.get())
                    on_change=Callback::new(move |c: String| set_new_color.set(c))
                    testid=Some("new-tag-color")
                />
                <input
                    type="text"
                    class="input input-bordered input-sm flex-1 min-w-40"
                    placeholder=move_tr!("tags-new-placeholder")
                    data-testid="new-tag-input"
                    prop:value=move || new_name.get()
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            on_create_root.run(());
                        }
                    }
                />
                <button
                    type="button"
                    class="btn btn-primary btn-sm"
                    data-testid="create-tag-btn"
                    on:click=move |_| on_create_root.run(())
                >
                    {move_tr!("common-add")}
                </button>
            </div>

            <div class="mb-2 flex items-center gap-2">
                <label class="input input-sm flex flex-1 items-center gap-2">
                    <Icon name="search" size=14 class="text-base-content/40" />
                    <input
                        type="search"
                        class="grow bg-transparent outline-none"
                        placeholder=move || i18n.tr("tags-search-placeholder")
                        data-testid="tag-search"
                        prop:value=move || query.get()
                        on:input=move |ev| set_query.set(event_target_value(&ev))
                    />
                </label>
                {move || {
                    (!query.get().trim().is_empty()).then(|| view! {
                        <IconButton
                            icon="x"
                            label=move_tr!("common-clear")
                            testid=Some("tag-search-clear")
                            on_click=Callback::new(move |_| set_query.set(String::new()))
                        />
                    })
                }}
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    let Some(result) = tags_res.get() else {
                        return view! {}.into_any();
                    };
                    let (tags, usage) = match result {
                        Ok(data) => data,
                        Err(e) => {
                            return view! {
                                <p class="text-error" data-testid="tags-error">
                                    {move || i18n.tr("error-prefix")} {e.to_string()}
                                </p>
                            }
                                .into_any();
                        }
                    };
                    if tags.is_empty() {
                        return view! {
                            <div
                                class="py-8 text-center text-base-content/50"
                                data-testid="tags-empty-state"
                            >
                                {move_tr!("tags-empty")}
                            </div>
                        }
                            .into_any();
                    }

                    let filtered = filter_tags(&tags, &query.get());
                    if filtered.is_empty() {
                        return view! {
                            <div
                                class="py-8 text-center text-base-content/50"
                                data-testid="tags-no-match"
                            >
                                {move_tr!("tags-no-match")}
                            </div>
                        }
                            .into_any();
                    }

                    // Delete needs the tag's name, its subtree size and its usage; both
                    // are already in this render, so the dialog needs no extra query.
                    let tags_for_delete = tags.clone();
                    let usage_for_delete = usage.clone();
                    let on_delete = Callback::new(move |id: String| {
                        let name = tags_for_delete
                            .iter()
                            .find(|tag| tag.id == id)
                            .map(|tag| tag.name.clone())
                            .unwrap_or_default();
                        let subtags = get_descendant_ids(&tags_for_delete, &id).len();
                        let usage = usage_for_delete.get(&id).copied().unwrap_or_default();
                        pending_delete.set(Some(PendingDelete {
                            id,
                            name,
                            subtags,
                            usage,
                        }));
                    });

                    let tree = build_tag_tree(&filtered);
                    view! {
                        <div class="flex flex-col gap-0.5" data-testid="tag-tree">
                            {tree
                                .into_iter()
                                .map(|node| {
                                    view! {
                                        <TagTreeRow
                                            node=node
                                            collapsed=collapsed
                                            usage=usage.clone()
                                            on_create_child=on_create_child
                                            on_rename=on_rename
                                            on_delete=on_delete
                                        />
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                        .into_any()
                }}
            </Suspense>

            {move || {
                pending_delete
                    .get()
                    .map(|pending| {
                        let message = if pending.subtags == 0 && pending.usage.total() == 0 {
                            move_tr!("tags-delete-confirm-plain", {
                                "name" => pending.name.clone()
                            })
                        } else {
                            move_tr!("tags-delete-confirm", {
                                "name" => pending.name.clone(),
                                "subtags" => pending.subtags,
                                "items" => pending.usage.items,
                                "lists" => pending.usage.lists,
                            })
                        };
                        view! {
                            <ConfirmModal
                                open=Signal::derive(|| true)
                                title=move_tr!("tags-delete-title").get()
                                message=message.get()
                                confirm_label=move_tr!("common-delete").get()
                                variant=ConfirmVariant::Danger
                                on_close=Callback::new(move |_| pending_delete.set(None))
                                on_confirm=on_delete_confirmed
                            />
                        }
                    })
            }}
        </div>
    }
}
