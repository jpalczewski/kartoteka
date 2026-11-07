use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use reactive_stores::Store;

use kartoteka_shared::types::DateItem;
use kartoteka_shared::{ChangeTopic, TagUsage};

use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::editable_text::EditableText;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_compact::{CompactLayout, CompactTestIds, ItemCompact};
use crate::components::items::item_groups::{ListGroup, group_ids};
use crate::components::items::item_view::ItemCaps;
use crate::components::tags::color_picker::TagColorPicker;
use crate::components::tags::tag_tree::{
    TagTreeRow, build_breadcrumb, build_subtree, get_descendant_ids, tag_type_key,
};
use crate::components::ui::{Chip, EntityIcon, Icon, SectionHeader};
use crate::server_fns::tags::{
    delete_tag, get_all_tags, get_tag_detail_data, get_tag_items, get_tag_usage_counts, merge_tags,
    set_tag_icon, update_tag, update_tag_color,
};
use crate::state::invalidation::{mutate, mutate_with, use_invalidation};
use crate::state::item_store::{ItemRows, patch_rows, tracked_items};

const ITEM_TESTIDS: CompactTestIds = CompactTestIds {
    row: None,
    toggle: None,
    title: Some("tag-item-link"),
    description: None,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailAction {
    Move,
    Merge,
}

/// Tag detail page: header (colour, name, icon, type), actions, subtags and the
/// lists/items this tag is attached to.
#[component]
pub fn TagDetailPage() -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let params = use_params_map();
    let tag_id_fn = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let navigate = StoredValue::new(use_navigate());

    let invalidation = use_invalidation();
    let (recursive, set_recursive) = signal(true);
    let active_action = RwSignal::new(Option::<DetailAction>::None);
    let (icon_draft, set_icon_draft) = signal(String::new());
    let confirm_delete = RwSignal::new(false);

    let all_tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags]),
        |_| get_all_tags(),
    );
    let usage_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists]),
        |_| get_tag_usage_counts(),
    );
    // Lists linked to this tag — the server already returns them together with the tag.
    let lists_res = Resource::new(
        move || {
            (
                tag_id_fn.get(),
                invalidation.track(&[ChangeTopic::Tags, ChangeTopic::Lists]),
            )
        },
        |(id, _)| get_tag_detail_data(id),
    );
    let items_res = Resource::new(
        move || {
            (
                tag_id_fn.get(),
                recursive.get(),
                invalidation.track(&[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists]),
            )
        },
        |(id, rec, _)| get_tag_items(id, rec),
    );

    // Item rows live in a keyed store, so a refetch (e.g. a live update) updates the
    // changed rows in place instead of re-creating every row.
    let item_rows = Store::new(ItemRows::<DateItem>::default());
    // Groups by list, sorted by list id and name as the page has always shown them.
    let item_groups = Memo::new(move |_| {
        let mut groups = group_ids(&tracked_items(item_rows));
        groups.sort_by(|a, b| (&a.list_id, &a.list_name).cmp(&(&b.list_id, &b.list_name)));
        groups
    });
    let has_items = move || !item_groups.with(Vec::is_empty);
    // Whether the last items load failed. A signal rather than a read of `items_res`, so the
    // list is not torn down while the resource refetches.
    let items_failed = RwSignal::new(false);

    // Rename / colour / icon / delete / move / merge callbacks — each reads the route
    // param through the Memo so they stay valid if the route changes.
    let on_rename = Callback::new(move |new_name: String| {
        if new_name.trim().is_empty() {
            return;
        }
        let id = tag_id_fn.get_untracked();
        mutate(
            update_tag(id, Some(new_name), None, None, false),
            &[ChangeTopic::Tags],
        );
    });

    let on_color_change = Callback::new(move |new_color: String| {
        let id = tag_id_fn.get_untracked();
        mutate(update_tag_color(id, new_color), &[ChangeTopic::Tags]);
    });

    let on_icon_save = Callback::new(move |_: ()| {
        let value = icon_draft.get_untracked().trim().to_string();
        let id = tag_id_fn.get_untracked();
        mutate(
            set_tag_icon(id, (!value.is_empty()).then_some(value)),
            &[ChangeTopic::Tags],
        );
    });

    let on_icon_clear = Callback::new(move |_: ()| {
        set_icon_draft.set(String::new());
        let id = tag_id_fn.get_untracked();
        mutate(set_tag_icon(id, None), &[ChangeTopic::Tags]);
    });

    let on_delete_click = Callback::new(move |_: ()| confirm_delete.set(true));
    let on_delete_confirmed = Callback::new(move |_: ()| {
        confirm_delete.set(false);
        let id = tag_id_fn.get_untracked();
        mutate_with(
            delete_tag(id),
            &[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists],
            move |()| navigate.with_value(|nav| nav("/tags", Default::default())),
        );
    });

    let on_move_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        let id = tag_id_fn.get_untracked();
        let clear = value.is_empty();
        let new_parent = if clear { None } else { Some(value) };
        mutate(
            update_tag(id, None, None, new_parent, clear),
            &[ChangeTopic::Tags],
        );
        active_action.set(None);
    };

    let on_merge_change = move |ev: leptos::ev::Event| {
        let value = event_target_value(&ev);
        if value.is_empty() {
            return;
        }
        let source = tag_id_fn.get_untracked();
        let target = value;
        let target_for_nav = target.clone();
        mutate_with(
            merge_tags(source, target),
            &[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists],
            move |_| {
                navigate.with_value(|nav| {
                    nav(&format!("/tags/{target_for_nav}"), Default::default());
                });
            },
        );
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <A
                href="/tags"
                attr:class="mb-2 inline-flex items-center gap-1 text-sm text-base-content/50 hover:text-primary"
            >
                <Icon name="arrow-left" size=14 />
                {move_tr!("tags-title")}
            </A>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    let Some(tags_result) = all_tags_res.get() else {
                        return view! {}.into_any();
                    };
                    let all_tags = match tags_result {
                        Ok(tags) => tags,
                        Err(e) => {
                            return view! {
                                <p class="text-error" data-testid="tag-error">
                                    {e.to_string()}
                                </p>
                            }
                                .into_any();
                        }
                    };
                    let tid = tag_id_fn.get();
                    let Some(current) = all_tags.iter().find(|t| t.id == tid).cloned() else {
                        return view! {
                            <p class="text-base-content/60" data-testid="tag-error">
                                {move_tr!("tags-not-found")}
                            </p>
                        }
                            .into_any();
                    };

                    let tag_id_str = current.id.clone();
                    let tag_name = current.name.clone();
                    let tag_color = current
                        .color
                        .clone()
                        .unwrap_or_else(|| kartoteka_shared::DEFAULT_TAG_COLOR.to_string());
                    let tag_icon = current.icon.clone();
                    let tag_type = current.tag_type.clone();
                    let usage = usage_res
                        .get()
                        .and_then(Result::ok)
                        .and_then(|counts| counts.get(&tag_id_str).copied())
                        .unwrap_or_default();

                    let breadcrumb = build_breadcrumb(&all_tags, &tag_id_str);
                    let subtree = build_subtree(&all_tags, &tag_id_str);
                    let descendant_ids = get_descendant_ids(&all_tags, &tag_id_str);

                    let move_targets: Vec<(String, String)> = all_tags
                        .iter()
                        .filter(|t| t.id != tag_id_str && !descendant_ids.contains(&t.id))
                        .map(|t| (t.id.clone(), t.name.clone()))
                        .collect();
                    let merge_targets: Vec<(String, String)> = all_tags
                        .iter()
                        .filter(|t| t.id != tag_id_str)
                        .map(|t| (t.id.clone(), t.name.clone()))
                        .collect();

                    let usage_for_tree: HashMap<String, TagUsage> =
                        usage_res.get().and_then(Result::ok).unwrap_or_default();
                    let collapsed: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

                    view! {
                        <div>
                            {(breadcrumb.len() > 1).then(|| view! {
                                <div class="mb-2 flex flex-wrap items-center gap-1 text-sm text-base-content/50">
                                    {breadcrumb.iter().enumerate().map(|(i, bt)| {
                                        let is_last = i == breadcrumb.len() - 1;
                                        let bt_id = bt.id.clone();
                                        let bt_name = bt.name.clone();
                                        if is_last {
                                            view! { <span class="font-semibold">{bt_name}</span> }.into_any()
                                        } else {
                                            view! {
                                                <span>
                                                    <A href=format!("/tags/{bt_id}") attr:class="link link-hover">{bt_name}</A>
                                                    " › "
                                                </span>
                                            }.into_any()
                                        }
                                    }).collect_view()}
                                </div>
                            })}

                            // ── Header: colour, icon, name, type ──────────────────
                            <div class="mb-3 flex items-start gap-3">
                                <span
                                    class="mt-1 h-8 w-8 shrink-0 rounded-full border border-base-300"
                                    style=format!("background: {tag_color};")
                                />
                                <div class="min-w-0 flex-1">
                                    <div class="flex items-center gap-2">
                                        <EntityIcon
                                            raw=tag_icon.clone()
                                            fallback="tag"
                                            size=20
                                            class="text-base-content/70"
                                        />
                                        <EditableText
                                            value=tag_name
                                            on_save=on_rename
                                            testid="tag-detail-name"
                                            class="text-2xl font-bold cursor-pointer hover:underline decoration-dotted"
                                        />
                                    </div>
                                    <div class="mt-1 flex flex-wrap items-center gap-2">
                                        <Chip icon="tag">{move || i18n.tr(tag_type_key(&tag_type))}</Chip>
                                        {move || {
                                            (usage.total() > 0).then(|| view! {
                                                <span
                                                    class="text-xs text-base-content/50"
                                                    data-testid="tag-detail-usage"
                                                >
                                                    {move_tr!("tags-usage-summary", {
                                                        "items" => usage.items,
                                                        "lists" => usage.lists,
                                                    })}
                                                </span>
                                            })
                                        }}
                                    </div>
                                </div>
                            </div>

                            <details
                                class="mb-3 rounded-md border border-base-300/60 px-2 py-1"
                                open=true
                            >
                                <summary class="cursor-pointer text-xs text-base-content/50">
                                    {move_tr!("tags-detail-appearance")}
                                </summary>
                                <div class="flex flex-col gap-2 py-2">
                                    <TagColorPicker
                                        value=Signal::derive({
                                            let tag_color = tag_color.clone();
                                            move || tag_color.clone()
                                        })
                                        on_change=on_color_change
                                        testid=Some("tag-detail-color")
                                    />
                                    <div class="flex items-center gap-2">
                                        <input
                                            type="text"
                                            class="input input-bordered input-xs flex-1"
                                            placeholder=move || {
                                                tag_icon
                                                    .clone()
                                                    .unwrap_or_else(|| i18n.tr("tags-icon-placeholder"))
                                            }
                                            data-testid="tag-icon-input"
                                            prop:value=move || icon_draft.get()
                                            on:input=move |ev| set_icon_draft.set(event_target_value(&ev))
                                            on:keydown=move |ev| {
                                                if ev.key() == "Enter" {
                                                    on_icon_save.run(());
                                                }
                                            }
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-xs"
                                            data-testid="tag-icon-set-btn"
                                            on:click=move |_| on_icon_save.run(())
                                        >
                                            {move_tr!("common-save")}
                                        </button>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            data-testid="tag-icon-clear-btn"
                                            on:click=move |_| {
                                                set_icon_draft.set(String::new());
                                                on_icon_clear.run(());
                                            }
                                        >
                                            {move_tr!("common-clear")}
                                        </button>
                                    </div>
                                </div>
                            </details>

                            // ── Actions ───────────────────────────────────────────
                            <div class="mb-4 flex flex-wrap gap-1">
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs"
                                    data-testid="tag-move-btn"
                                    on:click=move |_| active_action.update(|a| {
                                        *a = if *a == Some(DetailAction::Move) { None } else { Some(DetailAction::Move) };
                                    })
                                >
                                    <Icon name="folder-open" size=14 />
                                    {move_tr!("tags-move-button")}
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs"
                                    data-testid="tag-merge-btn"
                                    on:click=move |_| active_action.update(|a| {
                                        *a = if *a == Some(DetailAction::Merge) { None } else { Some(DetailAction::Merge) };
                                    })
                                >
                                    <Icon name="copy" size=14 />
                                    {move_tr!("tags-merge-button")}
                                </button>
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs text-error"
                                    data-testid="tag-delete-btn"
                                    on:click=move |_| on_delete_click.run(())
                                >
                                    <Icon name="trash" size=14 />
                                    {move_tr!("tags-delete-button")}
                                </button>
                            </div>

                            {move || match active_action.get() {
                                Some(DetailAction::Move) => view! {
                                    <div class="mb-4 flex items-center gap-2">
                                        <select
                                            class="select select-bordered select-sm"
                                            data-testid="tag-move-select"
                                            on:change=on_move_change
                                        >
                                            <option value="" selected=true>
                                                {move_tr!("tags-no-parent-option")}
                                            </option>
                                            {move_targets.clone().into_iter().map(|(id, name)| {
                                                view! { <option value=id>{name}</option> }
                                            }).collect_view()}
                                        </select>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            on:click=move |_| active_action.set(None)
                                        >
                                            {move_tr!("common-cancel")}
                                        </button>
                                    </div>
                                }.into_any(),
                                Some(DetailAction::Merge) => view! {
                                    <div class="mb-4 flex items-center gap-2">
                                        <select
                                            class="select select-bordered select-sm"
                                            data-testid="tag-merge-select"
                                            on:change=on_merge_change
                                        >
                                            <option value="" selected=true>
                                                {move_tr!("tags-merge-select-option")}
                                            </option>
                                            {merge_targets.clone().into_iter().map(|(id, name)| {
                                                view! { <option value=id>{name}</option> }
                                            }).collect_view()}
                                        </select>
                                        <button
                                            type="button"
                                            class="btn btn-ghost btn-xs"
                                            on:click=move |_| active_action.set(None)
                                        >
                                            {move_tr!("common-cancel")}
                                        </button>
                                    </div>
                                }.into_any(),
                                None => view! {}.into_any(),
                            }}

                            // ── Subtags ───────────────────────────────────────────
                            {(!subtree.is_empty()).then(|| view! {
                                <div class="mb-6">
                                    <SectionHeader
                                        title=move_tr!("tags-subtree-title")
                                        icon="list"
                                        count=subtree.len()
                                        class="mb-2"
                                    />
                                    <div class="flex flex-col gap-0.5">
                                        {subtree.into_iter().map(|node| view! {
                                            <TagTreeRow
                                                node=node
                                                collapsed=collapsed
                                                usage=usage_for_tree.clone()
                                                show_add_child=false
                                                show_rename=false
                                                show_delete=false
                                            />
                                        }).collect_view()}
                                    </div>
                                </div>
                            })}

                            // ── Lists tagged with this tag ────────────────────────
                            <div class="mb-6">
                                <SectionHeader title=move_tr!("tags-lists-title") icon="list" class="mb-2" />
                                <Suspense fallback=|| view! {}.into_any()>
                                    {move || lists_res.get().map(|result| match result {
                                        Err(e) => view! {
                                            <p class="text-error text-sm">{e.to_string()}</p>
                                        }.into_any(),
                                        Ok(data) => {
                                            if data.linked_lists.is_empty() {
                                                return view! {
                                                    <p
                                                        class="text-sm text-base-content/50"
                                                        data-testid="tag-no-lists"
                                                    >
                                                        {move_tr!("tags-no-lists")}
                                                    </p>
                                                }
                                                    .into_any();
                                            }
                                            view! {
                                                <ul class="flex flex-col gap-1" data-testid="tag-lists">
                                                    {data.linked_lists.into_iter().map(|list| {
                                                        view! {
                                                            <li>
                                                                <A
                                                                    href=format!("/lists/{}", list.id)
                                                                    attr:class="flex items-center gap-2 rounded-md px-1 py-1 text-sm hover:bg-base-200/50"
                                                                    attr:data-testid="tag-list-link"
                                                                >
                                                                    <EntityIcon
                                                                        raw=list.icon.clone()
                                                                        fallback=kartoteka_shared::default_list_icon(&list.list_type)
                                                                        size=14
                                                                        class="text-base-content/60"
                                                                    />
                                                                    <span class="truncate">{list.name.clone()}</span>
                                                                </A>
                                                            </li>
                                                        }
                                                    }).collect_view()}
                                                </ul>
                                            }
                                                .into_any()
                                        }
                                    })}
                                </Suspense>
                            </div>
                        </div>
                    }.into_any()
                }}
            </Suspense>

            // ── Items tagged with this tag ────────────────────────
            <div class="mb-4">
                <SectionHeader
                    title=move_tr!("tags-items-title")
                    icon="circle-check"
                    class="mb-2"
                >
                    <label class="flex cursor-pointer items-center gap-1.5">
                        <input
                            type="checkbox"
                            class="toggle toggle-xs toggle-primary"
                            data-testid="tag-recursive-toggle"
                            prop:checked=move || recursive.get()
                            on:change=move |ev| set_recursive.set(event_target_checked(&ev))
                        />
                        <span class="text-xs text-base-content/60">
                            {move_tr!("tags-include-subtags")}
                        </span>
                    </label>
                </SectionHeader>

                <Transition fallback=|| view! { <LoadingSpinner/> }>
                    // Patch the store first: this closure runs before the list below in the
                    // same render pass, on the server and in the browser.
                    {move || match items_res.get() {
                        Some(Ok(items)) => {
                            items_failed.set(false);
                            patch_rows(item_rows, items);
                            ().into_any()
                        }
                        Some(Err(e)) => {
                            items_failed.set(true);
                            view! {
                                <p class="text-error" data-testid="tag-error">
                                    {e.to_string()}
                                </p>
                            }
                            .into_any()
                        }
                        None => ().into_any(),
                    }}

                    <Show when=move || !items_failed.get()>
                        <Show
                            when=has_items
                            fallback=|| view! {
                                <p class="py-8 text-center text-base-content/50" data-testid="tag-no-items">
                                    {move_tr!("tags-no-items")}
                                </p>
                            }
                        >
                            <div data-testid="tag-items">
                                <TagItemGroups store=item_rows groups=item_groups/>
                            </div>
                        </Show>
                    </Show>
                </Transition>
            </div>

            <ConfirmModal
                open=Signal::derive(move || confirm_delete.get())
                title=move_tr!("tags-delete-title").get()
                message=move_tr!("tags-delete-message").get()
                confirm_label=move_tr!("common-delete").get()
                variant=ConfirmVariant::Danger
                on_close=Callback::new(move |_| confirm_delete.set(false))
                on_confirm=on_delete_confirmed
            />
        </div>
    }
}

/// List groups of the tag's items, keyed by list (id and name), each with its rows keyed
/// by item id.
#[component]
fn TagItemGroups(store: Store<ItemRows<DateItem>>, groups: Memo<Vec<ListGroup>>) -> impl IntoView {
    view! {
        <For
            each=move || groups.get()
            key=|group| (group.list_id.clone(), group.list_name.clone())
            let:group
        >
            {
                let list_id = group.list_id.clone();
                let item_ids = move || {
                    groups.with(|gs| {
                        gs.iter()
                            .find(|g| g.list_id == list_id)
                            .map(|g| g.item_ids.clone())
                            .unwrap_or_default()
                    })
                };
                view! {
                    <div class="mb-4">
                        <h4 class="mb-1 text-xs font-semibold uppercase tracking-wide text-base-content/50">
                            <A href=format!("/lists/{}", group.list_id) attr:class="link link-hover">
                                {group.list_name.clone()}
                            </A>
                        </h4>
                        <ul class="flex flex-col">
                            <For each=item_ids key=|id| id.clone() let:id>
                                <li>
                                    <ItemCompact
                                        store=store
                                        id=id
                                        caps=ItemCaps::NONE
                                        layout=CompactLayout::Plain
                                        testids=ITEM_TESTIDS
                                    />
                                </li>
                            </For>
                        </ul>
                    </div>
                }
            }
        </For>
    }
}
