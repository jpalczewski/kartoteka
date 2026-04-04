use crate::api;
use crate::api::client::GlooClient;
use crate::app::{ToastContext, ToastKind};
use crate::components::common::breadcrumbs::{BreadcrumbCrumb, Breadcrumbs};
use crate::components::common::editable_description::EditableDescription;
use crate::components::common::editable_title::EditableTitle;
use crate::components::common::inline_confirm_button::InlineConfirmButton;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::date_editor::DateEditor;
use crate::components::items::quantity_stepper::QuantityStepper;
use kartoteka_shared::*;
use leptos::prelude::*;
use leptos_router::hooks::{use_navigate, use_params_map};

fn section_card(title: &'static str, body: impl IntoView) -> impl IntoView {
    view! {
        <section class="rounded-[1.75rem] border border-base-300/80 bg-base-200/75 shadow-[0_24px_70px_-42px_rgba(0,0,0,0.9)] backdrop-blur-sm">
            <div class="p-5 sm:p-6">
                <div class="mb-4 flex items-center gap-3">
                    <div class="h-px flex-1 bg-base-300/80"></div>
                    <h2 class="text-sm font-semibold uppercase tracking-[0.2em] text-base-content/55">
                        {title}
                    </h2>
                    <div class="h-px w-8 bg-primary/45"></div>
                </div>
                {body.into_view()}
            </div>
        </section>
    }
}

fn detail_panel(
    label: &'static str,
    tone_class: &'static str,
    body: impl IntoView,
) -> impl IntoView {
    view! {
        <div class=format!("rounded-[1.35rem] border bg-base-100/90 p-4 shadow-[0_16px_40px_-28px_rgba(0,0,0,0.9)] {tone_class}")>
            <div class="mb-3 flex items-center gap-2 text-sm font-semibold text-base-content/78">
                <span class="inline-flex h-2.5 w-2.5 rounded-full bg-current opacity-70"></span>
                <span>{label}</span>
            </div>
            {body.into_view()}
        </div>
    }
}

/// Helper: spawn an update_item call with toast feedback.
fn spawn_save(
    client: GlooClient,
    list_id: String,
    item_id: String,
    toast: ToastContext,
    req: UpdateItemRequest,
) {
    leptos::task::spawn_local(async move {
        match api::update_item(&client, &list_id, &item_id, &req).await {
            Ok(_) => toast.push("Zapisano".into(), ToastKind::Success),
            Err(e) => toast.push(format!("Błąd: {e}"), ToastKind::Error),
        }
    });
}

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let toast = expect_context::<ToastContext>();
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let navigate = use_navigate();

    let list_id = move || params.with(|p| p.get("list_id").unwrap_or_default().to_string());
    let item_id = move || params.with(|p| p.get("id").unwrap_or_default().to_string());

    let detail_resource = {
        let client = client.clone();
        LocalResource::new(move || {
            let lid = list_id();
            let iid = item_id();
            let client = client.clone();
            async move { api::fetch_item_detail(&client, &lid, &iid).await }
        })
    };

    view! {
        <div class="mx-auto w-full max-w-4xl px-5 py-6 sm:px-8 lg:px-10">
            <Suspense fallback=move || {
                view! {
                    <div class="flex min-h-64 items-center justify-center px-2">
                        <LoadingSpinner />
                    </div>
                }
            }>
                {move || {
                    let result = detail_resource.get();

                    match result {
                        Some(Ok(detail)) => {
                            let item = detail.item;
                            let lid = list_id();
                            let list_name = detail.list_name;
                            let features = detail.list_features;

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

                            let crumbs: Vec<BreadcrumbCrumb> =
                                vec![(list_name.clone(), format!("/lists/{lid}"))];

                            let completed = RwSignal::new(item.completed);

                            let lid_for_delete = list_id();
                            let iid_for_delete = item_id();
                            let toast_del = toast.clone();
                            let nav = navigate.clone();
                            let client_del = client.clone();

                            let client_toggle = client.clone();
                            let client_title = client.clone();
                            let client_desc = client.clone();
                            let client_start = client.clone();
                            let client_deadline = client.clone();
                            let client_hard = client.clone();
                            let client_qty = client.clone();

                            let title_value = item.title.clone();
                            let description_value = item.description.clone();
                            let start_date = item.start_date.clone();
                            let start_time = item.start_time.clone();
                            let deadline_date = item.deadline.clone();
                            let deadline_time = item.deadline_time.clone();
                            let hard_deadline = item.hard_deadline.clone();
                            let quantity_target = item.quantity.unwrap_or(0);
                            let quantity_actual = item.actual_quantity.unwrap_or(0);
                            let quantity_unit = item.unit.clone().unwrap_or_default();

                            view! {
                                <div class="space-y-6 pb-8">
                                    <Breadcrumbs crumbs=crumbs />

                                    <section class="relative overflow-hidden rounded-[2rem] border border-primary/18 bg-linear-to-br from-base-200 via-base-200/96 to-primary/10 shadow-[0_34px_100px_-48px_rgba(0,0,0,0.95)]">
                                        <div class="pointer-events-none absolute inset-y-0 right-0 w-48 bg-linear-to-l from-primary/12 to-transparent blur-2xl"></div>
                                        <div class="relative p-6 sm:p-8">
                                            <div class="flex flex-col gap-5">
                                                <div class="min-w-0 flex-1">
                                                    <div class="mb-4 flex flex-wrap items-center gap-2">
                                                        <a
                                                            href=format!("/lists/{lid}")
                                                            class="badge badge-outline h-8 border-primary/35 px-3 font-medium text-primary hover:border-primary hover:bg-primary/10"
                                                        >
                                                            {list_name}
                                                        </a>
                                                        <span
                                                            class=move || {
                                                                if completed.get() {
                                                                    "badge badge-success h-8 px-3 font-medium"
                                                                } else {
                                                                    "badge h-8 border border-base-300 bg-base-100/70 px-3 font-medium text-base-content/72"
                                                                }
                                                            }
                                                        >
                                                            {move || if completed.get() { "Zrobione" } else { "W toku" }}
                                                        </span>
                                                    </div>

                                                    <div class="flex items-start gap-4">
                                                        <div class="rounded-[1.35rem] border border-primary/45 bg-primary/8 p-3 shadow-inner shadow-primary/10">
                                                            <input
                                                                type="checkbox"
                                                                class="checkbox checkbox-secondary checkbox-lg"
                                                                checked=item.completed
                                                                on:change=move |_| {
                                                                    let new_val = !completed.get();
                                                                    completed.set(new_val);
                                                                    spawn_save(
                                                                        client_toggle.clone(),
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
                                                        </div>
                                                        <div class="min-w-0 flex-1 pt-1">
                                                            <div class="mb-2 text-xs font-semibold uppercase tracking-[0.22em] text-base-content/45">
                                                                "Element"
                                                            </div>
                                                            <div
                                                                class=move || {
                                                                    if completed.get() {
                                                                        "line-through opacity-45".to_string()
                                                                    } else {
                                                                        String::new()
                                                                    }
                                                                }
                                                            >
                                                                <EditableTitle
                                                                    value=title_value
                                                                    class="block text-4xl leading-tight font-bold break-words".to_string()
                                                                    on_save=Callback::new(move |new_title: String| {
                                                                        spawn_save(
                                                                            client_title.clone(),
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
                                                            <p class="mt-3 max-w-2xl text-sm leading-6 text-base-content/62">
                                                                "Edytuj tytuł, status i szczegóły elementu w jednym miejscu."
                                                            </p>
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        </div>
                                    </section>

                                    {section_card(
                                        "Opis",
                                        view! {
                                            <div class="rounded-[1.35rem] border border-dashed border-base-300/80 bg-base-100/72 p-5">
                                                <EditableDescription
                                                    value=description_value
                                                    on_save=Callback::new(move |new_desc: Option<String>| {
                                                        spawn_save(
                                                            client_desc.clone(),
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
                                            </div>
                                        },
                                    )}

                                    {section_card(
                                        "Daty",
                                        view! {
                                            <div class="grid gap-4 lg:grid-cols-2">
                                                {if cfg_start {
                                                    detail_panel(
                                                        "Data rozpoczęcia",
                                                        "border-info/45",
                                                        view! {
                                                            <DateEditor
                                                                border_color="border-info"
                                                                initial_date=start_date
                                                                initial_time=start_time
                                                                has_time=true
                                                                on_change=Callback::new(
                                                                    move |(date, time): (String, Option<String>)| {
                                                                        let (d, t) = if date.is_empty() {
                                                                            (Some(None), Some(None))
                                                                        } else {
                                                                            (Some(Some(date)), time.map(Some))
                                                                        };
                                                                        spawn_save(
                                                                            client_start.clone(),
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
                                                        },
                                                    ).into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}

                                                {if cfg_deadline {
                                                    detail_panel(
                                                        "Termin",
                                                        "border-warning/45",
                                                        view! {
                                                            <DateEditor
                                                                border_color="border-warning"
                                                                initial_date=deadline_date
                                                                initial_time=deadline_time
                                                                has_time=true
                                                                on_change=Callback::new(
                                                                    move |(date, time): (String, Option<String>)| {
                                                                        let (d, t) = if date.is_empty() {
                                                                            (Some(None), Some(None))
                                                                        } else {
                                                                            (Some(Some(date)), time.map(Some))
                                                                        };
                                                                        spawn_save(
                                                                            client_deadline.clone(),
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
                                                        },
                                                    ).into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}

                                                {if cfg_hard {
                                                    let no_time: Option<String> = None;
                                                    detail_panel(
                                                        "Twardy termin",
                                                        "border-error/45",
                                                        view! {
                                                            <DateEditor
                                                                border_color="border-error"
                                                                initial_date=hard_deadline
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
                                                                            client_hard.clone(),
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
                                                        },
                                                    ).into_any()
                                                } else {
                                                    view! {}.into_any()
                                                }}
                                            </div>
                                        },
                                    )}

                                    {if has_quantity {
                                        section_card(
                                            "Ilość",
                                            view! {
                                                <div class="rounded-[1.35rem] border border-base-300/80 bg-base-100/86 p-5 shadow-[0_16px_40px_-28px_rgba(0,0,0,0.9)]">
                                                    <div class="mb-3 text-sm text-base-content/60">
                                                        "Postęp względem celu elementu"
                                                    </div>
                                                    <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                                                        <div>
                                                            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-base-content/42">
                                                                "Cel"
                                                            </div>
                                                            <div class="mt-1 text-2xl font-semibold">
                                                                {quantity_target}
                                                                {if quantity_unit.is_empty() { "".to_string() } else { format!(" {quantity_unit}") }}
                                                            </div>
                                                        </div>
                                                        <QuantityStepper
                                                            target=quantity_target
                                                            initial_actual=quantity_actual
                                                            unit=quantity_unit
                                                            on_change=Callback::new(move |new_val: i32| {
                                                                spawn_save(
                                                                    client_qty.clone(),
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
                                                </div>
                                            },
                                        ).into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}

                                    {section_card(
                                        "Usuń element",
                                        view! {
                                            <div class="flex flex-col gap-4 rounded-[1.35rem] border border-error/30 bg-linear-to-r from-error/12 to-transparent p-5 sm:flex-row sm:items-center sm:justify-between">
                                                <div class="space-y-1">
                                                    <p class="text-lg font-semibold text-error">
                                                        "Ta operacja jest nieodwracalna"
                                                    </p>
                                                    <p class="max-w-xl text-sm leading-6 text-base-content/62">
                                                        "Element zostanie usunięty z listy i nie będzie można go przywrócić."
                                                    </p>
                                                </div>
                                                <InlineConfirmButton
                                                    on_confirm=Callback::new(move |()| {
                                                        let lid = lid_for_delete.clone();
                                                        let iid = iid_for_delete.clone();
                                                        let toast = toast_del.clone();
                                                        let nav = nav.clone();
                                                        let client = client_del.clone();
                                                        leptos::task::spawn_local(async move {
                                                            match api::delete_item(&client, &lid, &iid).await {
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
                                        },
                                    )}
                                </div>
                            }
                            .into_any()
                        }
                        Some(Err(e)) => {
                            view! {
                                <div class="space-y-4 px-2">
                                    <div class="rounded-[1.75rem] border border-error/30 bg-error/10 p-6 shadow-[0_24px_70px_-42px_rgba(0,0,0,0.9)]">
                                        <div>
                                            <h2 class="text-lg font-semibold text-error">
                                                "Nie udało się załadować elementu"
                                            </h2>
                                            <p class="mt-2 text-sm text-base-content/70">{format!("Błąd: {e}")}</p>
                                        </div>
                                    </div>
                                </div>
                            }
                            .into_any()
                        }
                        None => {
                            view! {
                                <div class="flex min-h-64 items-center justify-center px-2">
                                    <LoadingSpinner />
                                </div>
                            }
                            .into_any()
                        }
                    }
                }}
            </Suspense>
        </div>
    }
}
