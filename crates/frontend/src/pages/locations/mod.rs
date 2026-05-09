use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::tags::{
    create_location, delete_tag, get_all_tags, update_location_metadata,
};
use kartoteka_shared::types::Tag;

const LOCATION_TYPES: &[&str] = &["country", "city", "address"];

fn extract_address(tag: &Tag) -> Option<String> {
    let meta = tag.metadata.as_deref()?;
    let v: serde_json::Value = serde_json::from_str(meta).ok()?;
    v.get("address")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from)
}

#[derive(Clone)]
struct LocNode {
    tag: Tag,
    children: Vec<LocNode>,
}

fn build_location_tree(tags: &[Tag]) -> Vec<LocNode> {
    fn collect(tags: &[Tag], parent_id: Option<&str>) -> Vec<LocNode> {
        tags.iter()
            .filter(|t| {
                LOCATION_TYPES.contains(&t.tag_type.as_str())
                    && t.parent_tag_id.as_deref() == parent_id
            })
            .map(|t| LocNode {
                tag: t.clone(),
                children: collect(tags, Some(&t.id)),
            })
            .collect()
    }
    collect(tags, None)
}

#[component]
pub fn LocationsPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);
    let tags_res = Resource::new(move || refresh.get(), |_| get_all_tags());

    let (new_country, set_new_country) = signal(String::new());

    let on_add_country = Callback::new(move |_: ()| {
        let name = new_country.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        set_new_country.set(String::new());
        leptos::task::spawn_local(async move {
            match create_location("country".into(), name, None, None).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">{move_tr!("locations-title")}</h2>

            <div class="flex gap-2 mb-6">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    prop:placeholder=move_tr!("locations-country-placeholder")
                    prop:value=move || new_country.get()
                    on:input=move |ev| set_new_country.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            on_add_country.run(());
                        }
                    }
                />
                <button
                    type="button"
                    class="btn btn-primary"
                    on:click=move |_| on_add_country.run(())
                >
                    {move_tr!("locations-add-country")}
                </button>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    tags_res
                        .get()
                        .map(|result| match result {
                            Err(e) => {
                                view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }
                                    .into_any()
                            }
                            Ok(tags) => {
                                let tree = build_location_tree(&tags);
                                if tree.is_empty() {
                                    return view! {
                                        <div class="text-center text-base-content/50 py-8">
                                            {move_tr!("locations-empty")}
                                        </div>
                                    }
                                        .into_any();
                                }
                                view! {
                                    <div class="flex flex-col gap-1">
                                        {tree
                                            .into_iter()
                                            .map(|node| {
                                                view! {
                                                    <LocationNode
                                                        node=node
                                                        depth=0
                                                        on_refresh=Callback::new(move |_: ()| {
                                                            set_refresh.update(|n| *n += 1)
                                                        })
                                                    />
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn LocationNode(node: LocNode, depth: usize, on_refresh: Callback<()>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();
    let tag = node.tag.clone();
    let tag_id = tag.id.clone();
    let tag_type = tag.tag_type.clone();

    let (show_add_child, set_show_add_child) = signal(false);
    let (show_edit, set_show_edit) = signal(false);
    let show_delete = RwSignal::new(false);
    let (child_name, set_child_name) = signal(String::new());
    let (child_address, set_child_address) = signal(String::new());
    let (edit_name, set_edit_name) = signal(tag.name.clone());
    let (edit_address, set_edit_address) = signal(extract_address(&tag).unwrap_or_default());

    let child_type: Option<&'static str> = match tag_type.as_str() {
        "country" => Some("city"),
        "city" => Some("address"),
        _ => None,
    };

    let indent_class = match depth {
        0 => "",
        1 => "ml-4",
        _ => "ml-8",
    };

    let on_add_child = {
        let tag_id = tag_id.clone();
        Callback::new(move |_: ()| {
            let Some(ct) = child_type else { return };
            let name = child_name.get_untracked();
            if name.trim().is_empty() {
                return;
            }
            let address = if ct == "address" {
                Some(child_address.get_untracked())
            } else {
                None
            };
            set_child_name.set(String::new());
            set_child_address.set(String::new());
            set_show_add_child.set(false);
            let parent_id = tag_id.clone();
            leptos::task::spawn_local(async move {
                match create_location(ct.to_string(), name, Some(parent_id), address).await {
                    Ok(_) => on_refresh.run(()),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_save_edit = {
        let tag_id = tag_id.clone();
        let is_address = tag_type == "address";
        Callback::new(move |_: ()| {
            let name = edit_name.get_untracked();
            if name.trim().is_empty() {
                return;
            }
            let address = if is_address {
                Some(edit_address.get_untracked())
            } else {
                None
            };
            set_show_edit.set(false);
            let id = tag_id.clone();
            leptos::task::spawn_local(async move {
                match update_location_metadata(id, Some(name), address, false).await {
                    Ok(_) => on_refresh.run(()),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_delete_confirm = {
        let tag_id = tag_id.clone();
        Callback::new(move |_: ()| {
            show_delete.set(false);
            let id = tag_id.clone();
            leptos::task::spawn_local(async move {
                match delete_tag(id).await {
                    Ok(_) => on_refresh.run(()),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let formal_address = extract_address(&tag);
    let display_name = tag.name.clone();
    let is_address_type = tag_type == "address";

    view! {
        <div class=format!("card bg-base-200 p-2 mb-1 {}", indent_class)>
            {move || {
                if show_edit.get() {
                    view! {
                        <div class="flex gap-2 items-center flex-wrap">
                            <input
                                type="text"
                                class="input input-bordered input-sm flex-1 min-w-32"
                                prop:value=move || edit_name.get()
                                on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                            />
                            {if is_address_type {
                                view! {
                                    <input
                                        type="text"
                                        class="input input-bordered input-sm flex-1 min-w-32"
                                        prop:placeholder=move_tr!(
                                            "locations-formal-address-placeholder"
                                        )
                                        prop:value=move || edit_address.get()
                                        on:input=move |ev| {
                                            set_edit_address.set(event_target_value(&ev))
                                        }
                                    />
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}
                            <button
                                class="btn btn-sm btn-primary"
                                on:click=move |_| on_save_edit.run(())
                            >
                                {move_tr!("locations-save")}
                            </button>
                            <button
                                class="btn btn-sm btn-ghost"
                                on:click=move |_| set_show_edit.set(false)
                            >
                                {move_tr!("locations-cancel")}
                            </button>
                        </div>
                    }
                        .into_any()
                } else {
                    view! {
                        <div class="flex gap-2 items-center">
                            <span class="flex-1 font-medium">{display_name.clone()}</span>
                            {formal_address
                                .as_ref()
                                .map(|a| {
                                    view! {
                                        <span class="text-sm text-base-content/60">{a.clone()}</span>
                                    }
                                })}
                            {if child_type.is_some() {
                                view! {
                                    <button
                                        class="btn btn-xs btn-ghost"
                                        on:click=move |_| {
                                            set_show_add_child.update(|v| *v = !*v)
                                        }
                                    >
                                        "+"
                                    </button>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}
                            <button
                                class="btn btn-xs btn-ghost"
                                on:click=move |_| set_show_edit.set(true)
                            >
                                "✎"
                            </button>
                            <button
                                class="btn btn-xs btn-ghost text-error"
                                on:click=move |_| show_delete.set(true)
                            >
                                "✕"
                            </button>
                        </div>
                    }
                        .into_any()
                }
            }}

            <Show when=move || show_add_child.get()>
                <div class="flex gap-2 mt-2 ml-4 flex-wrap">
                    <input
                        type="text"
                        class="input input-bordered input-sm flex-1 min-w-32"
                        prop:placeholder=move || match child_type {
                            Some("city") => i18n.tr("locations-city-placeholder"),
                            _ => i18n.tr("locations-alias-placeholder"),
                        }
                        prop:value=move || child_name.get()
                        on:input=move |ev| set_child_name.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" {
                                on_add_child.run(());
                            }
                        }
                    />
                    {if child_type == Some("address") {
                        view! {
                            <input
                                type="text"
                                class="input input-bordered input-sm flex-1 min-w-32"
                                prop:placeholder=move_tr!("locations-formal-address-placeholder")
                                prop:value=move || child_address.get()
                                on:input=move |ev| set_child_address.set(event_target_value(&ev))
                            />
                        }
                            .into_any()
                    } else {
                        view! {}.into_any()
                    }}
                    <button
                        class="btn btn-sm btn-primary"
                        on:click=move |_| on_add_child.run(())
                    >
                        {move || match child_type {
                            Some("city") => i18n.tr("locations-add-city"),
                            _ => i18n.tr("locations-add-address"),
                        }}
                    </button>
                    <button
                        class="btn btn-sm btn-ghost"
                        on:click=move |_| set_show_add_child.set(false)
                    >
                        {move_tr!("locations-cancel")}
                    </button>
                </div>
            </Show>

            <ConfirmModal
                open=Signal::from(show_delete)
                title="Usuń lokalizację".to_string()
                message=i18n.tr("locations-delete-confirm")
                confirm_label="Usuń".to_string()
                variant=ConfirmVariant::Danger
                on_confirm=on_delete_confirm
                on_close=Callback::new(move |_: ()| show_delete.set(false))
            />

            {node
                .children
                .into_iter()
                .map(|child| {
                    view! {
                        <LocationNode node=child depth=depth + 1 on_refresh=on_refresh/>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}
