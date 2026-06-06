use crate::app::{ToastContext, ToastKind};
use crate::components::common::dnd::{DragHandleButton, ItemDropTargetMarker};
use crate::components::items::item_row::ItemRow;
use crate::components::lists::add_input::AddInput;
use crate::server_fns::items::{create_item, delete_item, get_list_data, move_item, toggle_item};
use crate::state::dnd::{DndState, EntityKind, ItemDndState, ItemDropTarget};
use kartoteka_shared::types::{Item, List};
use leptos::prelude::*;

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
    let list_id = StoredValue::new(sublist.id.clone());
    let list_name = sublist.name.clone();

    let items = RwSignal::new(initial_items);

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
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
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

    let on_delete = Callback::new(move |item_id: String| {
        let lid = list_id.get_value();
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => {
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
                <a
                    href=format!("/lists/{}", sublist.id)
                    class="relative z-[2] font-semibold hover:underline decoration-dotted"
                    data-testid="sublist-open-link"
                    on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    {list_name}
                    <span class="ml-1 text-xs opacity-50">"↗"</span>
                </a>
                {move || {
                    let current = items.get();
                    let done = current.iter().filter(|i| i.completed).count();
                    let total = current.len();
                    view! {
                        <span class="text-sm text-base-content/60 ml-auto mr-4" data-testid="sublist-progress">
                            {done} "/" {total}
                        </span>
                    }
                }}
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
                    view! {
                        <div class="flex flex-col gap-1">
                            {sorted
                                .into_iter()
                                .map(|item| {
                                    let mt = targets.clone();
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
