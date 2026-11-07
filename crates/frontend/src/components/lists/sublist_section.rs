use crate::app::{ToastContext, ToastKind};
use crate::components::common::dnd::{DragHandleButton, ItemDropTargetMarker};
use crate::components::items::item_row::{DateSavePayload, ItemRow};
use crate::components::lists::add_input::AddInput;
use crate::components::ui::{ActionMenu, ActionMenuItem, CompletionBar, Icon};
use crate::server_fns::items::{
    create_item, delete_item, get_list_data, move_item, toggle_item, update_actual_quantity,
    update_item_dates, update_item_description,
};
use crate::state::dnd::{DndState, EntityKind, ItemDndState, ItemDropTarget};
use crate::state::invalidation::{mutate_with, use_invalidation};
use kartoteka_shared::ChangeTopic;
use kartoteka_shared::FEATURE_QUANTITY;
use kartoteka_shared::types::{Item, List};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

#[component]
pub fn SublistSection(
    sublist: List,
    /// Pre-fetched items from the parent's ListData — avoids nested Resource hydration.
    initial_items: Vec<Item>,
    on_any_change: Callback<()>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    #[prop(optional)] dnd_state: Option<RwSignal<DndState>>,
    #[prop(optional)] item_dnd_state: Option<RwSignal<ItemDndState>>,
    #[prop(optional)] on_item_drop: Option<Callback<ItemDropTarget>>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let invalidation = use_invalidation();
    let list_id = StoredValue::new(sublist.id.clone());
    let list_name = sublist.name.clone();
    let navigate = use_navigate();
    let open_href = format!("/lists/{}", sublist.id);
    let menu_open = RwSignal::new(false);

    // Rows are driven by the *sublist's* own features, not the parent list's: a sublist
    // created as "checklist" must render checkboxes even under a quantity parent.
    let list_features: Vec<String> = sublist
        .features
        .iter()
        .map(|f| f.feature_name.clone())
        .collect();
    let has_quantity = list_features.iter().any(|f| f == FEATURE_QUANTITY);

    let items = RwSignal::new(initial_items);

    // Refetch this sublist's rows after a mutation so the progress bar and the row state follow.
    let refresh_items = move || {
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            if let Ok(data) = get_list_data(lid).await {
                items.set(data.items);
            }
        });
    };

    let on_add = Callback::new(move |title: String| {
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            match create_item(lid.clone(), title, None, None, None, None, None, None).await {
                Ok(_) => {
                    if let Ok(data) = get_list_data(lid).await {
                        items.set(data.items);
                    }
                    on_any_change.run(());
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_toggle = Callback::new(move |item_id: String| {
        mutate_with(toggle_item(item_id), &[ChangeTopic::Items], move |_| {
            refresh_items();
            on_any_change.run(());
        });
    });

    let on_quantity_change = Callback::new(move |(item_id, new_actual): (String, i32)| {
        mutate_with(
            update_actual_quantity(item_id, new_actual),
            &[ChangeTopic::Items],
            move |_| refresh_items(),
        );
    });

    let on_description_save = Callback::new(move |(item_id, description): (String, String)| {
        mutate_with(
            update_item_description(item_id, description),
            &[ChangeTopic::Items],
            move |_| refresh_items(),
        );
    });

    let on_date_save = Callback::new(move |payload: DateSavePayload| {
        let (item_id, start, start_time, deadline, deadline_time, hard_deadline) = payload;
        mutate_with(
            update_item_dates(
                item_id,
                start,
                start_time,
                deadline,
                deadline_time,
                hard_deadline,
            ),
            &[ChangeTopic::Items],
            move |_| refresh_items(),
        );
    });

    let on_delete = Callback::new(move |item_id: String| {
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => {
                    invalidation.invalidate(ChangeTopic::Items);
                    if let Ok(data) = get_list_data(lid).await {
                        items.set(data.items);
                    }
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_move_item = Callback::new(move |(item_id, target_list_id): (String, String)| {
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            match move_item(item_id, target_list_id).await {
                Ok(_) => {
                    if let Ok(data) = get_list_data(lid).await {
                        items.set(data.items);
                    }
                    on_any_change.run(());
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let move_targets_stored = StoredValue::new(move_targets);

    view! {
        <div
            class="collapse collapse-arrow bg-base-200 mb-2"
            data-testid="sublist-section"
        >
            <input type="checkbox" checked=true />
            <div class="collapse-title font-semibold flex items-center gap-2">
                {dnd_state.map(|state| {
                    let sid = sublist.id.clone();
                    view! {
                        <DragHandleButton dnd_state=state kind=EntityKind::List dragged_id=sid aria_label="Przeciągnij podlistę" />
                    }
                })}
                // The title stays a real link (primary open action, used by e2e);
                // the secondary "open" action lives in the shared action menu.
                <a
                    href=format!("/lists/{}", sublist.id)
                    class="relative z-[2] font-semibold hover:underline decoration-dotted"
                    data-testid="sublist-open-link"
                    on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    {list_name}
                    <Icon name="external-link" size=12 class="ml-1 opacity-50" />
                </a>
                {move || {
                    let current = items.get();
                    let done = current.iter().filter(|i| i.completed).count();
                    let total = current.len();
                    view! {
                        <span class="ml-auto mr-4 w-32" data-testid="sublist-progress">
                            <CompletionBar done=done total=total />
                        </span>
                    }
                }}
                <ActionMenu
                    items=vec![
                        ActionMenuItem::new(
                            "external-link",
                            move_tr!("lists-preview-open-full"),
                            Callback::new(move |_: ()| {
                                navigate(&open_href, Default::default());
                            }),
                        )
                            .testid("sublist-action-open"),
                    ]
                    open=menu_open
                    label=move_tr!("common-actions")
                    testid=Some("sublist-menu")
                    class="relative z-[3]"
                />
            </div>
            <div class="collapse-content">
                {move || {
                    let current = items.get();
                    let mut sorted = current;
                    sorted.sort_by(|a, b| {
                        a.completed
                            .cmp(&b.completed)
                            .then(a.position.cmp(&b.position))
                    });
                    let targets = move_targets_stored.get_value();
                    let features = list_features.clone();
                    view! {
                        <div class="flex flex-col gap-1">
                            {sorted
                                .into_iter()
                                .map(|item| {
                                    let mt = targets.clone();
                                    let features = features.clone();
                                    let iid = item.id.clone();
                                    match item_dnd_state.zip(on_item_drop) {
                                        Some((state, cb)) => {
                                            let before_tgt = ItemDropTarget::before(
                                                sublist.id.clone(),
                                                iid,
                                            );
                                            view! {
                                                <ItemDropTargetMarker
                                                    dnd_state=state
                                                    target=before_tgt
                                                    on_drop=cb
                                                />
                                                <ItemRow
                                                    item=item
                                                    on_toggle=on_toggle
                                                    on_delete=on_delete
                                                    list_features=features
                                                    has_quantity=has_quantity
                                                    on_quantity_change=on_quantity_change
                                                    on_description_save=on_description_save
                                                    on_date_save=on_date_save
                                                    move_targets=mt
                                                    on_move=on_move_item
                                                    dnd_state=state
                                                />
                                            }
                                            .into_any()
                                        }
                                        None => view! {
                                            <ItemRow
                                                item=item
                                                on_toggle=on_toggle
                                                on_delete=on_delete
                                                list_features=features
                                                has_quantity=has_quantity
                                                on_quantity_change=on_quantity_change
                                                on_description_save=on_description_save
                                                on_date_save=on_date_save
                                                move_targets=mt
                                                on_move=on_move_item
                                            />
                                        }
                                        .into_any(),
                                    }
                                })
                                .collect::<Vec<_>>()}
                            {item_dnd_state.zip(on_item_drop).map(|(state, cb)| {
                                view! {
                                    <ItemDropTargetMarker
                                        dnd_state=state
                                        target=ItemDropTarget::end(sublist.id.clone())
                                        on_drop=cb
                                        label="Upuść na koniec"
                                    />
                                }
                            })}
                            <div class="mt-2">
                                <AddInput
                                    placeholder=Signal::derive(|| "Nowy element...".to_string())
                                    button_label=Signal::derive(|| "Dodaj".to_string())
                                    testid="sublist-new-item"
                                    on_submit=on_add
                                />
                            </div>
                        </div>
                    }
                }}
            </div>
        </div>
    }
}
