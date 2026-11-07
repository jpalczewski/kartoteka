use crate::components::lists::add_input::AddInput;
use crate::components::lists::appearance::{AppearanceTarget, appearance_menu_item};
use crate::components::ui::{ActionMenu, ActionMenuItem, EntityIcon, Icon};
use crate::server_fns::items::{create_item, toggle_item};
use crate::server_fns::lists::get_list_preview_items;
use crate::state::invalidation::{mutate, use_invalidation};
use kartoteka_shared::ChangeTopic;
use kartoteka_shared::default_list_icon;
use kartoteka_shared::types::List;
use kartoteka_shared::{FEATURE_CHECKLIST, FEATURE_QUANTITY, PreviewItem};
use leptos::prelude::*;
use leptos_fluent::move_tr;

#[component]
pub fn ListPreview(
    list: List,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_archive: Option<Callback<String>>,
    #[prop(optional)] force_expand: Option<Signal<bool>>,
) -> impl IntoView {
    let invalidation = use_invalidation();
    let has_checklist = list.has_feature(FEATURE_CHECKLIST);
    let has_quantity = list.has_feature(FEATURE_QUANTITY);
    let list_id = list.id.clone();
    let list_name = list.name.clone();
    let icon = list.icon.clone();
    let color = list.color.clone();
    let icon_fallback = default_list_icon(&list.list_type);
    let href = format!("/lists/{}", list.id);

    let list_id_del = list.id.clone();
    let list_id_archive = list.id.clone();
    let list_archived = list.archived;
    let menu_open = RwSignal::new(false);
    let mut menu_items = Vec::new();
    menu_items.extend(appearance_menu_item(
        AppearanceTarget::list(&list),
        "list-preview-action-appearance",
    ));
    if let Some(cb) = on_archive {
        let lid = list_id_archive.clone();
        menu_items.push(
            ActionMenuItem::new(
                "archive",
                if list_archived {
                    move_tr!("lists-restore-aria")
                } else {
                    move_tr!("lists-archive-aria")
                },
                Callback::new(move |_: ()| cb.run(lid.clone())),
            )
            .testid("list-preview-action-archive"),
        );
    }
    if let Some(cb) = on_delete {
        let lid = list_id_del.clone();
        menu_items.push(
            ActionMenuItem::new(
                "trash",
                move_tr!("lists-header-delete-button"),
                Callback::new(move |_: ()| cb.run(lid.clone())),
            )
            .danger()
            .testid("list-preview-delete"),
        );
    }

    let (is_open, set_is_open) = signal(false);
    // `opened` gates the first fetch: a collapsed preview never queries.
    let (opened, set_opened) = signal(false);

    // When force_expand flips to true, open the panel and trigger the first fetch.
    // When it flips back to false, close the panel (collapse all).
    // Effects are client-only, which is correct — this is a user-triggered action.
    if let Some(force) = force_expand {
        Effect::new(move |_| {
            if force.get() {
                set_is_open.set(true);
                set_opened.set(true);
            } else {
                set_is_open.set(false);
            }
        });
    }

    let items_resource = Resource::new(
        {
            let lid = list_id.clone();
            move || {
                (
                    lid.clone(),
                    opened.get(),
                    invalidation.track(&[ChangeTopic::Items]),
                )
            }
        },
        |(id, opened, _)| async move {
            if opened {
                get_list_preview_items(id).await
            } else {
                Ok(Vec::<PreviewItem>::new())
            }
        },
    );

    let on_toggle = Callback::new(move |item_id: String| {
        mutate(toggle_item(item_id), &[ChangeTopic::Items]);
    });

    let lid_add = list_id.clone();
    let on_add = Callback::new(move |title: String| {
        let lid = lid_add.clone();
        mutate(
            create_item(lid, title, None, None, None, None, None, None),
            &[ChangeTopic::Items],
        );
    });

    view! {
        <div class="collapse collapse-arrow bg-base-200 border border-base-300">
            <input
                type="checkbox"
                prop:checked=move || is_open.get()
                on:change=move |ev| {
                    let checked = event_target_checked(&ev);
                    set_is_open.set(checked);
                    if checked {
                        set_opened.set(true);
                    }
                }
            />
            <div class="collapse-title font-semibold flex items-center gap-2 pr-10">
                <EntityIcon raw=icon fallback=icon_fallback color=color />
                <span data-testid="list-preview-title">{list_name}</span>
                <div class="ml-auto flex items-center gap-1 mr-2 relative z-10" on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()>
                    <a
                        href=href
                        class="btn btn-ghost btn-square btn-xs"
                        title=move_tr!("lists-preview-open-full")
                        aria-label=move_tr!("lists-preview-open-full")
                        data-testid="list-preview-open-link"
                        on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                    >
                        <Icon name="external-link" size=14 />
                    </a>
                    {(!menu_items.is_empty()).then(move || view! {
                        <ActionMenu
                            items=menu_items
                            open=menu_open
                            label=move_tr!("common-actions")
                            testid=Some("list-preview-menu")
                        />
                    })}
                </div>
            </div>
            <div class="collapse-content">
                <Suspense fallback=|| view! { <p class="text-sm text-base-content/50 py-2">{move_tr!("common-loading")}</p> }>
                    {move || items_resource.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="text-error text-sm">{e.to_string()}</p>
                        }.into_any(),
                        Ok(items) => {
                            if items.is_empty() && !opened.get() {
                                // Not yet expanded — render nothing (collapse is closed).
                                return view! {}.into_any();
                            }
                            view! {
                                <div class="flex flex-col gap-1 pt-1">
                                    {items.into_iter().map(|item| {
                                        let iid = item.id.clone();
                                        view! {
                                            <div class="flex items-start gap-2 py-1">
                                                {has_checklist.then(|| view! {
                                                    <input
                                                        type="checkbox"
                                                        class="checkbox checkbox-sm mt-0.5"
                                                        prop:checked=item.completed
                                                        on:change=move |_| on_toggle.run(iid.clone())
                                                    />
                                                })}
                                                <div class="flex-1 min-w-0">
                                                    <div class="flex items-center gap-1">
                                                        <span
                                                            class:line-through=move || has_checklist && item.completed
                                                            class:opacity-50=move || has_checklist && item.completed
                                                        >
                                                            {item.title.clone()}
                                                        </span>
                                                        {item.quantity.filter(|_| has_quantity).map(|q| {
                                                            let display = item.unit.as_deref()
                                                                .map(|u| format!("{q} {u}"))
                                                                .unwrap_or_else(|| format!("{q}×"));
                                                            view! {
                                                                <span class="text-xs text-base-content/50 ml-1">{display}</span>
                                                            }
                                                        })}
                                                        {item.has_comments.then(|| view! {
                                                            <span class="text-xs text-base-content/40">{"💬"}</span>
                                                        })}
                                                    </div>
                                                    {item.description.as_ref().map(|d| view! {
                                                        <p class="text-xs text-base-content/50 truncate">{d.clone()}</p>
                                                    })}
                                                </div>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                    <div class="mt-2">
                                        <AddInput
                                            placeholder=move_tr!("lists-preview-new-item-placeholder")
                                            button_label=move_tr!("common-add")
                                            on_submit=on_add
                                        />
                                    </div>
                                </div>
                            }.into_any()
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}
