use crate::api::{delete_item, fetch_item_detail, update_item};
use crate::app::{ToastContext, ToastKind};
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::editable_title::EditableTitle;
use crate::components::common::inline_confirm_button::InlineConfirmButton;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::date_editor::DateEditor;
use crate::components::items::quantity_stepper::QuantityStepper;
use kartoteka_shared::*;
use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

/// Helper: spawn an update_item call with toast feedback.
fn spawn_save(list_id: String, item_id: String, toast: ToastContext, req: UpdateItemRequest) {
    leptos::task::spawn_local(async move {
        match update_item(&list_id, &item_id, &req).await {
            Ok(_) => toast.push("Zapisano".into(), ToastKind::Success),
            Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
        }
    });
}

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let toast = expect_context::<ToastContext>();
    let navigate = leptos_router::hooks::use_navigate();

    let list_id = move || params.with(|p| p.get("list_id").unwrap_or_default().to_string());
    let item_id = move || params.with(|p| p.get("id").unwrap_or_default().to_string());

    // Single call returns item + list context
    let detail_resource = LocalResource::new(move || {
        let lid = list_id();
        let iid = item_id();
        async move { fetch_item_detail(&lid, &iid).await }
    });

    view! {
        <Suspense fallback=move || view! { <LoadingSpinner /> }>
            {move || {
                let result = detail_resource.get().map(|r| (*r).clone());

                match result {
                    Some(Ok(detail)) => {
                        let item = detail.item;
                        let lid = list_id();
                        let list_name = detail.list_name;
                        let features = detail.list_features;

                        // Deadlines config
                        let deadlines_config = features
                            .iter()
                            .find(|f| f.name == FEATURE_DEADLINES)
                            .map(|f| f.config.clone())
                            .unwrap_or(serde_json::Value::Null);
                        let has_quantity = features.iter().any(|f| f.name == FEATURE_QUANTITY);

                        let cfg_start = deadlines_config
                            .get("has_start_date")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let cfg_deadline = deadlines_config
                            .get("has_deadline")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let cfg_hard = deadlines_config
                            .get("has_hard_deadline")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        // Breadcrumbs (list name as link, item title as plain text)
                        let crumbs = vec![(list_name, format!("/lists/{lid}"))];
                        let item_title_for_crumb = item.title.clone();

                        // Completed toggle
                        let completed = RwSignal::new(item.completed);

                        // Delete
                        let lid_for_delete = list_id();
                        let iid_for_delete = item_id();
                        let toast_del = toast.clone();
                        let nav = navigate.clone();

                        view! {
                            // Breadcrumbs with item title as non-linked final crumb
                            <div class="breadcrumbs text-sm mb-4">
                                <ul>
                                    <li><a href="/">"Home"</a></li>
                                    {crumbs
                                        .into_iter()
                                        .map(|(label, href)| {
                                            view! { <li><a href=href>{label}</a></li> }
                                        })
                                        .collect::<Vec<_>>()}
                                    <li>{item_title_for_crumb}</li>
                                </ul>
                            </div>

                            // Completed checkbox + title
                            <div class="flex items-center gap-3 mb-4">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-secondary checkbox-lg"
                                    checked=item.completed
                                    on:change=move |_| {
                                        let new_val = !completed.get();
                                        completed.set(new_val);
                                        spawn_save(
                                            list_id(),
                                            item_id(),
                                            toast.clone(),
                                            UpdateItemRequest {
                                                completed: Some(new_val),
                                                ..Default::default()
                                            },
                                        );
                                    }
                                />
                                <EditableTitle
                                    value=item.title.clone()
                                    on_save=Callback::new(move |new_title: String| {
                                        spawn_save(
                                            list_id(),
                                            item_id(),
                                            toast.clone(),
                                            UpdateItemRequest {
                                                title: Some(new_title),
                                                ..Default::default()
                                            },
                                        );
                                    })
                                />
                            </div>

                            // Description (None from EditableDescription means "clear")
                            <EditableDescription
                                value=item.description.clone()
                                on_save=Callback::new(move |new_desc: Option<String>| {
                                    spawn_save(
                                        list_id(),
                                        item_id(),
                                        toast.clone(),
                                        UpdateItemRequest {
                                            description: Some(new_desc),
                                            ..Default::default()
                                        },
                                    );
                                })
                            />

                            // Dates section
                            {if cfg_start {
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">
                                            "Data rozpoczęcia"
                                        </label>
                                        <DateEditor
                                            border_color="border-info"
                                            initial_date=item.start_date.clone()
                                            initial_time=item.start_time.clone()
                                            has_time=true
                                            on_change=Callback::new(
                                                move |(date, time): (String, Option<String>)| {
                                                    let (d, t) = if date.is_empty() {
                                                        (Some(None), Some(None))
                                                    } else {
                                                        (Some(Some(date)), time.map(Some))
                                                    };
                                                    spawn_save(
                                                        list_id(),
                                                        item_id(),
                                                        toast.clone(),
                                                        UpdateItemRequest {
                                                            start_date: d,
                                                            start_time: t,
                                                            ..Default::default()
                                                        },
                                                    );
                                                },
                                            )
                                        />
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            {if cfg_deadline {
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Termin"</label>
                                        <DateEditor
                                            border_color="border-warning"
                                            initial_date=item.deadline.clone()
                                            initial_time=item.deadline_time.clone()
                                            has_time=true
                                            on_change=Callback::new(
                                                move |(date, time): (String, Option<String>)| {
                                                    let (d, t) = if date.is_empty() {
                                                        (Some(None), Some(None))
                                                    } else {
                                                        (Some(Some(date)), time.map(Some))
                                                    };
                                                    spawn_save(
                                                        list_id(),
                                                        item_id(),
                                                        toast.clone(),
                                                        UpdateItemRequest {
                                                            deadline: d,
                                                            deadline_time: t,
                                                            ..Default::default()
                                                        },
                                                    );
                                                },
                                            )
                                        />
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            {if cfg_hard {
                                let no_time: Option<String> = None;
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Twardy termin"</label>
                                        <DateEditor
                                            border_color="border-error"
                                            initial_date=item.hard_deadline.clone()
                                            initial_time=no_time
                                            has_time=false
                                            on_change=Callback::new(
                                                move |(date, _time): (String, Option<String>)| {
                                                    let d = if date.is_empty() {
                                                        Some(None)
                                                    } else {
                                                        Some(Some(date))
                                                    };
                                                    spawn_save(
                                                        list_id(),
                                                        item_id(),
                                                        toast.clone(),
                                                        UpdateItemRequest {
                                                            hard_deadline: d,
                                                            ..Default::default()
                                                        },
                                                    );
                                                },
                                            )
                                        />
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Quantity section
                            {if has_quantity {
                                let target = item.quantity.unwrap_or(0);
                                let unit = item.unit.clone().unwrap_or_default();
                                view! {
                                    <div class="mb-4">
                                        <label class="label text-sm font-medium">"Ilość"</label>
                                        <QuantityStepper
                                            target=target
                                            initial_actual=item.actual_quantity.unwrap_or(0)
                                            unit=unit
                                            on_change=Callback::new(move |new_val: i32| {
                                                spawn_save(
                                                    list_id(),
                                                    item_id(),
                                                    toast.clone(),
                                                    UpdateItemRequest {
                                                        actual_quantity: Some(new_val),
                                                        ..Default::default()
                                                    },
                                                );
                                            })
                                        />
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {}.into_any()
                            }}

                            // Delete button
                            <div class="mt-8 pt-4 border-t border-base-300">
                                <InlineConfirmButton
                                    on_confirm=Callback::new(move |()| {
                                        let lid = lid_for_delete.clone();
                                        let iid = iid_for_delete.clone();
                                        let toast = toast_del.clone();
                                        let nav = nav.clone();
                                        leptos::task::spawn_local(async move {
                                            match delete_item(&lid, &iid).await {
                                                Ok(_) => {
                                                    toast.push("Usunięto".into(), ToastKind::Success);
                                                    nav(&format!("/lists/{lid}"), Default::default());
                                                }
                                                Err(e) => {
                                                    toast.push(
                                                        format!("Błąd: {e}"),
                                                        ToastKind::Error,
                                                    )
                                                }
                                            }
                                        });
                                    })
                                    label="Usuń element".to_string()
                                    confirm_label="Na pewno usunąć?".to_string()
                                    class="btn btn-error btn-outline btn-sm".to_string()
                                    confirm_class="btn btn-error btn-sm".to_string()
                                />
                            </div>
                        }
                            .into_any()
                    }
                    Some(Err(e)) => {
                        view! { <p class="text-error">{format!("Błąd: {e}")}</p> }.into_any()
                    }
                    None => view! { <LoadingSpinner /> }.into_any(),
                }
            }}
        </Suspense>
    }
}
