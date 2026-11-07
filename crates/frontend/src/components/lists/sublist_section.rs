use crate::components::common::dnd::DragHandleButton;
use crate::components::items::item_list::{ItemDnd, ItemRowList};
use crate::components::items::item_view::ItemCaps;
use crate::components::lists::add_input::AddInput;
use crate::components::ui::{ActionMenu, ActionMenuItem, CompletionBar, Icon};
use crate::server_fns::items::create_item;
use crate::state::dnd::{DndState, EntityKind};
use crate::state::invalidation::mutate;
use crate::state::item_store::{ItemRows, tracked_items};
use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::Item;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;
use reactive_stores::Store;

/// A sublist as its section needs it. Its own features, not the parent list's, decide how its
/// rows look: a sublist created as "checklist" must render checkboxes even under a quantity
/// parent.
#[derive(Clone, Debug, PartialEq)]
pub struct SublistMeta {
    pub id: String,
    pub name: String,
    pub features: Vec<String>,
}

/// One sublist of the list page. Created once per sublist id; its rows come from `store`,
/// which the page patches on every refetch, so the section (and its add-item input) is not
/// re-created by live updates.
#[component]
pub fn SublistSection(
    id: String,
    sublists: Signal<Vec<SublistMeta>>,
    store: Store<ItemRows<Item>>,
    /// `(id, name)` of every list; the sublist itself is left out of its move menu.
    all_lists: Signal<Vec<(String, String)>>,
    dnd_state: RwSignal<DndState>,
    item_dnd: ItemDnd,
) -> impl IntoView {
    let navigate = use_navigate();
    let open_href = format!("/lists/{id}");
    let menu_open = RwSignal::new(false);

    let meta = Memo::new({
        let id = id.clone();
        move |_| sublists.with(|all| all.iter().find(|s| s.id == id).cloned())
    });
    let name = Memo::new(move |_| meta.with(|m| m.as_ref().map(|m| m.name.clone())));
    let caps = Memo::new(move |_| {
        let features = meta.with(|m| m.as_ref().map(|m| m.features.clone()));
        ItemCaps {
            drag: true,
            delete: true,
            move_to: true,
            ..ItemCaps::from_features(&features.unwrap_or_default())
        }
    });
    let move_targets = Memo::new({
        let id = id.clone();
        move |_| {
            all_lists.with(|lists| {
                lists
                    .iter()
                    .filter(|(lid, _)| *lid != id)
                    .cloned()
                    .collect::<Vec<_>>()
            })
        }
    });

    // Open items first, then by position.
    let ids = Memo::new(move |_| {
        let mut items = tracked_items(store);
        items.sort_by(|a, b| {
            a.completed
                .cmp(&b.completed)
                .then(a.position.cmp(&b.position))
        });
        items.into_iter().map(|i| i.id).collect::<Vec<_>>()
    });
    let progress = Memo::new(move |_| {
        let items = tracked_items(store);
        (items.iter().filter(|i| i.completed).count(), items.len())
    });

    let on_add = Callback::new({
        let id = id.clone();
        move |title: String| {
            let lid = id.clone();
            mutate(
                async move {
                    create_item(lid, title, None, None, None, None, None, None)
                        .await
                        .map(|_| ())
                },
                &[ChangeTopic::Items],
            );
        }
    });

    view! {
        <div
            class="collapse collapse-arrow bg-base-200 mb-2"
            data-testid="sublist-section"
        >
            <input type="checkbox" checked=true />
            <div class="collapse-title font-semibold flex items-center gap-2">
                <DragHandleButton
                    dnd_state=dnd_state
                    kind=EntityKind::List
                    dragged_id=id.clone()
                    aria_label="Przeciągnij podlistę"
                />
                // The title stays a real link (primary open action, used by e2e);
                // the secondary "open" action lives in the shared action menu.
                <a
                    href=open_href.clone()
                    class="relative z-[2] font-semibold hover:underline decoration-dotted"
                    data-testid="sublist-open-link"
                    on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    {move || name.get()}
                    <Icon name="external-link" size=12 class="ml-1 opacity-50" />
                </a>
                <span class="ml-auto mr-4 w-32" data-testid="sublist-progress">
                    {move || {
                        let (done, total) = progress.get();
                        view! { <CompletionBar done=done total=total /> }
                    }}
                </span>
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
                <div class="flex flex-col gap-1">
                    <ItemRowList
                        store=store
                        ids=ids
                        list_id=Signal::stored(id)
                        caps=caps
                        move_targets=move_targets
                        dnd=item_dnd
                    />
                    <div class="mt-2">
                        <AddInput
                            placeholder=Signal::derive(|| "Nowy element...".to_string())
                            button_label=Signal::derive(|| "Dodaj".to_string())
                            testid="sublist-new-item"
                            on_submit=on_add
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
