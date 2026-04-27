use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::{components::A, hooks::use_params_map};

use kartoteka_shared::{
    FEATURE_CHECKLIST, FEATURE_DEADLINES, FEATURE_LOCATION, FEATURE_QUANTITY, FEATURE_TIME_TRACKING,
};

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::date_field::DateFieldInput;
use crate::components::relations::RelatedEntities;
use crate::components::tags::tag_list::TagList;
use crate::components::time_entries::ItemTimerWidget;
use crate::server_fns::items::{
    get_item, toggle_item, update_item, update_item_dates, update_item_quantity,
};
use crate::server_fns::lists::get_list_feature_names;
use crate::server_fns::tags::{
    assign_tag_to_item, get_all_tags, get_item_tags, remove_tag_from_item,
};

fn toggle_item_tag(
    item_id: String,
    tag_id: String,
    is_assigned: bool,
    set_tag_refresh: WriteSignal<u32>,
) {
    leptos::task::spawn_local(async move {
        let result = if is_assigned {
            remove_tag_from_item(item_id, tag_id).await
        } else {
            assign_tag_to_item(item_id, tag_id).await
        };
        if let Err(e) = result {
            leptos::logging::warn!("tag toggle error: {e}");
        }
        set_tag_refresh.update(|n| *n += 1);
    });
}

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("list_id").unwrap_or_default();
    let item_id = move || params.read().get("id").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (refresh, set_refresh) = signal(0u32);
    let (tag_refresh, set_tag_refresh) = signal(0u32);

    let item_res = Resource::new(move || (item_id(), refresh.get()), |(id, _)| get_item(id));
    let item_tags_res = Resource::new(
        move || (item_id(), tag_refresh.get()),
        |(id, _)| get_item_tags(id),
    );
    let all_tags_res = Resource::new(move || tag_refresh.get(), |_| get_all_tags());
    let list_features_res = Resource::new(list_id, get_list_feature_names);

    // Derive a single features signal so each section doesn't repeat the lookup.
    let features = Signal::derive(move || {
        list_features_res
            .get()
            .and_then(|r| r.ok())
            .unwrap_or_default()
    });
    let has = move |name: &'static str| move || features.with(|fs| fs.iter().any(|f| f == name));

    let title_input: RwSignal<String> = RwSignal::new(String::new());
    let description_input: RwSignal<String> = RwSignal::new(String::new());
    let quantity_input: RwSignal<String> = RwSignal::new(String::new());
    let actual_quantity_input: RwSignal<String> = RwSignal::new(String::new());
    let unit_input: RwSignal<String> = RwSignal::new(String::new());
    let start_date_input: RwSignal<String> = RwSignal::new(String::new());
    let start_time_input: RwSignal<String> = RwSignal::new(String::new());
    let deadline_input: RwSignal<String> = RwSignal::new(String::new());
    let deadline_time_input: RwSignal<String> = RwSignal::new(String::new());
    let hard_deadline_input: RwSignal<String> = RwSignal::new(String::new());

    let on_save = move |_: leptos::ev::MouseEvent| {
        let id = item_id();
        let title = title_input.get();
        let description = description_input.get();
        leptos::task::spawn_local(async move {
            match update_item(id, title, description).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_save_dates = move |_: leptos::ev::MouseEvent| {
        let id = item_id();
        let sd = Some(start_date_input.get());
        let st = Some(start_time_input.get());
        let dl = Some(deadline_input.get());
        let dt = Some(deadline_time_input.get());
        let hd = Some(hard_deadline_input.get());
        leptos::task::spawn_local(async move {
            match update_item_dates(id, sd, st, dl, dt, hd).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_save_quantity = move |_: leptos::ev::MouseEvent| {
        let id = item_id();
        let qty = quantity_input.get().trim().parse::<i32>().ok();
        let actual_qty = actual_quantity_input.get().trim().parse::<i32>().ok();
        let unit = unit_input.get();
        leptos::task::spawn_local(async move {
            match update_item_quantity(id, qty, actual_qty, unit).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_toggle = move |_: leptos::ev::Event| {
        let id = item_id();
        leptos::task::spawn_local(async move {
            match toggle_item(id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let back_href = move || format!("/lists/{}", list_id());

    Effect::new(move |_| {
        if let Some(Ok(item)) = item_res.get() {
            title_input.set(item.title.clone());
            description_input.set(item.description.clone().unwrap_or_default());
            quantity_input.set(item.quantity.map(|q| q.to_string()).unwrap_or_default());
            actual_quantity_input.set(
                item.actual_quantity
                    .map(|q| q.to_string())
                    .unwrap_or_default(),
            );
            unit_input.set(item.unit.clone().unwrap_or_default());
            start_date_input.set(
                item.start_date
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
            );
            start_time_input.set(item.start_time.clone().unwrap_or_default());
            deadline_input.set(
                item.deadline
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
            );
            deadline_time_input.set(item.deadline_time.clone().unwrap_or_default());
            hard_deadline_input.set(
                item.hard_deadline
                    .as_ref()
                    .map(|d| d.to_string())
                    .unwrap_or_default(),
            );
        }
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="mb-4">
                <A href=back_href attr:class="btn btn-ghost btn-sm gap-1">
                    {move_tr!("item-back-to-list")}
                </A>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || item_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Error: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(item) => {
                        let completed = item.completed;
                        let created_at = item.created_at.clone();
                        let updated_at = item.updated_at.clone();

                        view! {
                            <div class="flex flex-col gap-4">
                                // Checklist toggle — only when list has checklist feature
                                <Show when=has(FEATURE_CHECKLIST)>
                                    <label class="flex items-center gap-3 cursor-pointer">
                                        <input
                                            type="checkbox"
                                            class="checkbox checkbox-primary"
                                            data-testid="item-detail-toggle"
                                            checked=completed
                                            on:change=on_toggle
                                        />
                                        <span class="text-base-content/70" data-testid="item-detail-status">
                                            {if completed {
                                                move_tr!("item-status-completed")
                                            } else {
                                                move_tr!("item-status-open")
                                            }}
                                        </span>
                                    </label>
                                </Show>

                                <div class="form-control">
                                    <label class="label">
                                        <span class="label-text font-semibold">{move_tr!("item-title-label")}</span>
                                    </label>
                                    <input
                                        type="text"
                                        class="input input-bordered w-full"
                                        data-testid="item-detail-title"
                                        prop:value=move || title_input.get()
                                        on:input=move |ev| title_input.set(event_target_value(&ev))
                                    />
                                </div>

                                <div class="form-control">
                                    <label class="label">
                                        <span class="label-text font-semibold">{move_tr!("item-description-label")}</span>
                                    </label>
                                    <textarea
                                        class="textarea textarea-bordered w-full h-32"
                                        data-testid="item-detail-description"
                                        prop:value=move || description_input.get()
                                        on:input=move |ev| description_input.set(event_target_value(&ev))
                                    />
                                </div>

                                <button
                                    type="button"
                                    class="btn btn-primary w-full"
                                    data-testid="item-detail-save"
                                    on:click=on_save
                                >
                                    {move_tr!("common-save")}
                                </button>

                                // Quantity section — only when list has quantity feature
                                <Show when=has(FEATURE_QUANTITY)>
                                    <div class="divider text-sm">{move_tr!("item-quantity-section")}</div>
                                    <div class="flex gap-2 items-end">
                                        <div class="form-control flex-1">
                                            <label class="label">
                                                <span class="label-text">{move_tr!("item-quantity-label")}</span>
                                            </label>
                                            <input
                                                type="number"
                                                class="input input-bordered w-full"
                                                data-testid="item-detail-quantity"
                                                prop:value=move || quantity_input.get()
                                                on:input=move |ev| quantity_input.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div class="form-control flex-1">
                                            <label class="label">
                                                <span class="label-text">{move_tr!("item-quantity-actual-label")}</span>
                                            </label>
                                            <input
                                                type="number"
                                                class="input input-bordered w-full"
                                                data-testid="item-detail-actual-quantity"
                                                prop:value=move || actual_quantity_input.get()
                                                on:input=move |ev| actual_quantity_input.set(event_target_value(&ev))
                                            />
                                        </div>
                                        <div class="form-control flex-1">
                                            <label class="label">
                                                <span class="label-text">{move_tr!("item-quantity-unit-label")}</span>
                                            </label>
                                            <input
                                                type="text"
                                                class="input input-bordered w-full"
                                                placeholder=move || move_tr!("item-quantity-unit-placeholder").get()
                                                data-testid="item-detail-unit"
                                                prop:value=move || unit_input.get()
                                                on:input=move |ev| unit_input.set(event_target_value(&ev))
                                            />
                                        </div>
                                    </div>
                                    <button
                                        type="button"
                                        class="btn btn-secondary w-full"
                                        data-testid="item-detail-save-quantity"
                                        on:click=on_save_quantity
                                    >
                                        {move_tr!("item-save-quantity")}
                                    </button>
                                </Show>

                                // Dates section — only when list has deadlines feature
                                <Show when=has(FEATURE_DEADLINES)>
                                    <div class="divider text-sm">{move_tr!("item-dates-section")}</div>
                                    <div class="flex flex-col gap-2">
                                        <DateFieldInput
                                            label=move_tr!("item-dates-start")
                                            value=start_date_input
                                            time_value=start_time_input
                                            data_testid="item-detail-start-date"
                                            show_clear=true
                                            show_quick=true
                                            large=true
                                        />
                                        <DateFieldInput
                                            label=move_tr!("item-dates-deadline")
                                            value=deadline_input
                                            time_value=deadline_time_input
                                            data_testid="item-detail-deadline"
                                            show_clear=true
                                            show_quick=true
                                            large=true
                                        />
                                        <DateFieldInput
                                            label=move_tr!("item-dates-hard-deadline")
                                            value=hard_deadline_input
                                            data_testid="item-detail-hard-deadline"
                                            show_clear=true
                                            show_quick=true
                                            large=true
                                        />
                                        <button
                                            type="button"
                                            class="btn btn-secondary w-full"
                                            data-testid="item-detail-save-dates"
                                            on:click=on_save_dates
                                        >
                                            {move_tr!("item-save-dates")}
                                        </button>
                                    </div>
                                </Show>

                                // Tags section
                                <div class="divider text-sm">{move_tr!("item-tags-section")}</div>
                                {move || {
                                    let item_tags = item_tags_res.get()
                                        .and_then(|r| r.ok())
                                        .unwrap_or_default();
                                    let all_tags = all_tags_res.get()
                                        .and_then(|r| r.ok())
                                        .unwrap_or_default();
                                    let tag_ids: Vec<String> = item_tags.iter().map(|t| t.id.clone()).collect();
                                    let iid = item_id();

                                    let location_types = ["country", "city", "address"];
                                    let general_tag_ids: Vec<String> = item_tags
                                        .iter()
                                        .filter(|t| !location_types.contains(&t.tag_type.as_str()))
                                        .map(|t| t.id.clone())
                                        .collect();
                                    let general_all_tags: Vec<kartoteka_shared::types::Tag> = all_tags
                                        .iter()
                                        .filter(|t| !location_types.contains(&t.tag_type.as_str()))
                                        .cloned()
                                        .collect();

                                    let on_tag_toggle = Callback::new(move |tid: String| {
                                        let is_assigned = tag_ids.contains(&tid);
                                        toggle_item_tag(iid.clone(), tid, is_assigned, set_tag_refresh);
                                    });

                                    view! {
                                        <TagList
                                            all_tags=general_all_tags
                                            selected_tag_ids=general_tag_ids
                                            on_toggle=on_tag_toggle
                                        />
                                    }
                                }}

                                // Location section — only when list has location feature
                                <Show when=has(FEATURE_LOCATION)>
                                    {move || {
                                        let item_tags = item_tags_res.get()
                                            .and_then(|r| r.ok())
                                            .unwrap_or_default();
                                        let all_tags = all_tags_res.get()
                                            .and_then(|r| r.ok())
                                            .unwrap_or_default();
                                        let iid = item_id();

                                        let location_types = ["country", "city", "address"];
                                        let location_tag_ids: Vec<String> = item_tags
                                            .iter()
                                            .filter(|t| location_types.contains(&t.tag_type.as_str()))
                                            .map(|t| t.id.clone())
                                            .collect();
                                        let location_all_tags: Vec<kartoteka_shared::types::Tag> = all_tags
                                            .into_iter()
                                            .filter(|t| location_types.contains(&t.tag_type.as_str()))
                                            .collect();
                                        let tag_ids_for_cb = item_tags.iter().map(|t| t.id.clone()).collect::<Vec<_>>();

                                        let on_location_toggle = Callback::new(move |tid: String| {
                                            let is_assigned = tag_ids_for_cb.contains(&tid);
                                            toggle_item_tag(iid.clone(), tid, is_assigned, set_tag_refresh);
                                        });

                                        view! {
                                            <div class="divider text-sm">"📍 " {move_tr!("lists-feature-location")}</div>
                                            <TagList
                                                all_tags=location_all_tags
                                                selected_tag_ids=location_tag_ids
                                                on_toggle=on_location_toggle
                                            />
                                        }
                                    }}
                                </Show>

                                <div class="text-xs text-base-content/40 mt-2 flex flex-col gap-1">
                                    <span>{move_tr!("item-created-label")} " " {created_at}</span>
                                    <span>{move_tr!("item-updated-label")} " " {updated_at}</span>
                                </div>

                                <CommentSection
                                    entity_type="item"
                                    entity_id=Signal::derive(item_id)
                                />

                                <RelatedEntities
                                    entity_id=Signal::derive(item_id)
                                />

                                // Time tracking — only when list has time_tracking feature
                                <Show when=has(FEATURE_TIME_TRACKING)>
                                    <ItemTimerWidget
                                        item_id=Signal::derive(item_id)
                                    />
                                </Show>
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
