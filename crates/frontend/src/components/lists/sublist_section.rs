use crate::app::{ToastContext, ToastKind};
use crate::components::common::dnd::{DragHandleButton, ItemDropTargetMarker};
use crate::components::items::item_row::ItemRow;
use crate::components::items::item_view::ItemCaps;
use crate::components::lists::add_input::AddInput;
use crate::components::ui::{ActionMenu, ActionMenuItem, CompletionBar, Icon};
use crate::server_fns::items::{create_item, get_list_data};
use crate::state::dnd::{DndState, EntityKind, ItemDndState, ItemDropTarget};
use crate::state::item_store::{ItemRows, patch_rows};
use kartoteka_shared::types::{Item, List};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;
use reactive_stores::Store;

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
                    // Rows read their item from a store; this section still rebuilds it per refetch.
                    let row_store = Store::new(ItemRows::<Item>::default());
                    let ids: Vec<String> = sorted.iter().map(|i| i.id.clone()).collect();
                    patch_rows(row_store, sorted);
                    let caps = Signal::stored(ItemCaps {
                        drag: item_dnd_state.is_some(),
                        delete: true,
                        move_to: true,
                        ..ItemCaps::from_features(&list_features)
                    });
                    let targets = Signal::stored(move_targets_stored.get_value());
                    view! {
                        <div class="flex flex-col gap-1">
                            {ids
                                .into_iter()
                                .map(|iid| {
                                    match item_dnd_state.zip(on_item_drop) {
                                        Some((state, cb)) => {
                                            let before_tgt = ItemDropTarget::before(
                                                sublist.id.clone(),
                                                iid.clone(),
                                            );
                                            view! {
                                                <ItemDropTargetMarker
                                                    dnd_state=state
                                                    target=before_tgt
                                                    on_drop=cb
                                                />
                                                <ItemRow
                                                    store=row_store
                                                    id=iid
                                                    caps=caps
                                                    move_targets=targets
                                                    dnd_state=Some(state)
                                                />
                                            }
                                            .into_any()
                                        }
                                        None => view! {
                                            <ItemRow
                                                store=row_store
                                                id=iid
                                                caps=caps
                                                move_targets=targets
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
