use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::state::dnd::{
    DndState, DraggedItem, DropTarget, ItemDndState, ItemDropTarget, begin_drag, begin_item_drag,
    clear_dnd_state, clear_item_dnd_state, is_dragged_id, is_dragged_item, is_hovered_item_target,
    is_hovered_target, set_hovered_item_target, set_hovered_target,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropMarkerKind {
    Before,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragHandleLabel {
    Reorder,
    ReorderGroup,
}

pub fn drag_shell_class(is_dragged: bool) -> &'static str {
    if is_dragged {
        "flex items-stretch gap-3 opacity-60 scale-[0.985] rotate-[0.4deg] transition-all duration-200"
    } else {
        "flex items-stretch gap-3 transition-all duration-200"
    }
}

pub fn drag_surface_class(is_dragged: bool, is_hovered: bool) -> &'static str {
    if is_dragged {
        "flex-1 rounded-[1.35rem] bg-base-100/80 ring-2 ring-primary/20 shadow-2xl transition-all duration-200"
    } else if is_hovered {
        "flex-1 rounded-[1.35rem] bg-primary/10 ring-2 ring-primary/35 shadow-lg shadow-primary/10 -translate-y-0.5 scale-[1.01] transition-all duration-150"
    } else {
        "flex-1 rounded-[1.35rem] transition-all duration-200"
    }
}

pub fn drag_handle_class(is_dragged: bool) -> &'static str {
    if is_dragged {
        "inline-flex h-11 w-10 shrink-0 items-center justify-center rounded-2xl border border-primary/35 bg-primary/15 text-primary shadow-sm cursor-grabbing transition-all duration-150"
    } else {
        "inline-flex h-11 w-10 shrink-0 items-center justify-center rounded-2xl border border-base-300 bg-base-100 text-base-content/45 shadow-sm cursor-grab transition-all duration-150 hover:-translate-y-0.5 hover:border-primary/35 hover:bg-primary/10 hover:text-primary"
    }
}

pub fn drop_marker_class(is_active: bool, is_hovered: bool) -> &'static str {
    if is_active && is_hovered {
        "group flex h-12 items-center gap-3 overflow-hidden rounded-2xl border border-primary/55 bg-primary/15 px-4 opacity-100 shadow-[0_18px_48px_-28px_rgba(16,185,129,0.8)] transition-all duration-150"
    } else if is_active {
        "group flex h-10 items-center gap-3 overflow-hidden rounded-2xl border border-primary/20 bg-base-100/80 px-3 opacity-100 transition-all duration-150 hover:border-primary/45 hover:bg-primary/10"
    } else {
        "pointer-events-none flex h-0 items-center gap-3 overflow-hidden rounded-2xl border border-transparent px-3 opacity-0 transition-all duration-150"
    }
}

pub fn drop_marker_line_class(is_active: bool, is_hovered: bool) -> &'static str {
    if is_active && is_hovered {
        "h-[2px] flex-1 bg-gradient-to-r from-transparent via-primary to-transparent transition-all duration-150"
    } else if is_active {
        "h-px flex-1 bg-gradient-to-r from-transparent via-primary/55 to-transparent transition-all duration-150 group-hover:via-primary"
    } else {
        "h-px flex-1 bg-transparent"
    }
}

pub fn drop_marker_label_class(is_active: bool, is_hovered: bool) -> &'static str {
    if is_active && is_hovered {
        "shrink-0 text-[11px] font-bold uppercase tracking-[0.28em] text-primary transition-all duration-150"
    } else if is_active {
        "shrink-0 text-[10px] font-semibold uppercase tracking-[0.24em] text-primary/65 transition-colors duration-150 group-hover:text-primary"
    } else {
        "hidden"
    }
}

fn configure_drag_over(event: &web_sys::DragEvent) {
    event.prevent_default();
    if let Some(data_transfer) = event.data_transfer() {
        data_transfer.set_drop_effect("move");
    }
}

fn configure_drag_start(event: &web_sys::DragEvent, dragged_id: &str) {
    if let Some(data_transfer) = event.data_transfer() {
        let _ = data_transfer.set_data("text/plain", dragged_id);
        data_transfer.set_effect_allowed("move");
    }
}

#[component]
pub fn ReorderDropTarget(
    dnd_state: RwSignal<DndState>,
    target: DropTarget,
    on_drop: Callback<DropTarget>,
) -> impl IntoView {
    let target_for_hover = target.clone();
    let target_for_dragover = target.clone();
    let target_for_drop = target.clone();
    let is_active = Signal::derive(move || dnd_state.with(|state| state.is_active()));
    let is_hovered =
        Signal::derive(move || dnd_state.with(|state| is_hovered_target(state, &target_for_hover)));
    let kind = if target.is_end() {
        DropMarkerKind::End
    } else {
        DropMarkerKind::Before
    };

    view! {
        <div
            class=move || drop_marker_class(is_active.get(), is_hovered.get())
            on:dragover=move |event: web_sys::DragEvent| {
                configure_drag_over(&event);
                dnd_state.update(|state| set_hovered_target(state, target_for_dragover.clone()));
            }
            on:drop=move |event: web_sys::DragEvent| {
                event.prevent_default();
                on_drop.run(target_for_drop.clone());
                dnd_state.update(clear_dnd_state);
            }
        >
            <span class=move || drop_marker_line_class(is_active.get(), is_hovered.get())></span>
            <span class=move || drop_marker_label_class(is_active.get(), is_hovered.get())>
                {move || match kind {
                    DropMarkerKind::Before => move_tr!("lists-dnd-drop-here").get(),
                    DropMarkerKind::End => move_tr!("lists-dnd-drop-at-end").get(),
                }}
            </span>
            <span class=move || drop_marker_line_class(is_active.get(), is_hovered.get())></span>
        </div>
    }
}

#[component]
pub fn DragShell(
    dnd_state: RwSignal<DndState>,
    dragged_id: String,
    children: Children,
) -> impl IntoView {
    let dragged_id_for_state = dragged_id;
    let is_dragged =
        Signal::derive(move || dnd_state.with(|state| is_dragged_id(state, &dragged_id_for_state)));

    view! {
        <div class=move || drag_shell_class(is_dragged.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn DragHandleButton(
    dnd_state: RwSignal<DndState>,
    dragged_id: String,
    label: DragHandleLabel,
    #[prop(default = "")] extra_class: &'static str,
) -> impl IntoView {
    let dragged_id_for_state = dragged_id.clone();
    let dragged_id_for_dragstart = dragged_id.clone();
    let is_dragged =
        Signal::derive(move || dnd_state.with(|state| is_dragged_id(state, &dragged_id_for_state)));

    view! {
        <button
            type="button"
            class=move || {
                let base_class = drag_handle_class(is_dragged.get());
                if extra_class.is_empty() {
                    base_class.to_string()
                } else {
                    format!("{base_class} {extra_class}")
                }
            }
            draggable="true"
            aria-label=move || match label {
                DragHandleLabel::Reorder => move_tr!("lists-dnd-reorder-aria").get(),
                DragHandleLabel::ReorderGroup => move_tr!("lists-dnd-reorder-group-aria").get(),
            }
            title=move || match label {
                DragHandleLabel::Reorder => move_tr!("lists-dnd-reorder-aria").get(),
                DragHandleLabel::ReorderGroup => move_tr!("lists-dnd-reorder-group-aria").get(),
            }
            on:dragstart=move |event: web_sys::DragEvent| {
                configure_drag_start(&event, &dragged_id_for_dragstart);
                dnd_state.update(|state| begin_drag(state, dragged_id_for_dragstart.clone()));
            }
            on:dragend=move |_| dnd_state.update(clear_dnd_state)
        >
            <DragGrip />
        </button>
    }
}

#[component]
pub fn DragSurface(
    dnd_state: RwSignal<DndState>,
    dragged_id: String,
    hover_target: DropTarget,
    children: Children,
) -> impl IntoView {
    let dragged_id_for_state = dragged_id;
    let hover_target_for_state = hover_target;
    let is_dragged =
        Signal::derive(move || dnd_state.with(|state| is_dragged_id(state, &dragged_id_for_state)));
    let is_hovered = Signal::derive(move || {
        dnd_state.with(|state| is_hovered_target(state, &hover_target_for_state))
    });

    view! {
        <div class=move || drag_surface_class(is_dragged.get(), is_hovered.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn ItemDropTargetMarker(
    dnd_state: RwSignal<ItemDndState>,
    target: ItemDropTarget,
    on_drop: Callback<ItemDropTarget>,
) -> impl IntoView {
    let target_for_hover = target.clone();
    let target_for_dragover = target.clone();
    let target_for_drop = target.clone();
    let is_active = Signal::derive(move || dnd_state.with(|state| state.is_active()));
    let is_hovered = Signal::derive(move || {
        dnd_state.with(|state| is_hovered_item_target(state, &target_for_hover))
    });
    let kind = if target.before_item_id().is_some() {
        DropMarkerKind::Before
    } else {
        DropMarkerKind::End
    };

    view! {
        <div
            class=move || drop_marker_class(is_active.get(), is_hovered.get())
            on:dragover=move |event: web_sys::DragEvent| {
                configure_drag_over(&event);
                dnd_state.update(|state| {
                    set_hovered_item_target(state, target_for_dragover.clone())
                });
            }
            on:drop=move |event: web_sys::DragEvent| {
                event.prevent_default();
                on_drop.run(target_for_drop.clone());
                dnd_state.update(clear_item_dnd_state);
            }
        >
            <span class=move || drop_marker_line_class(is_active.get(), is_hovered.get())></span>
            <span class=move || drop_marker_label_class(is_active.get(), is_hovered.get())>
                {move || match kind {
                    DropMarkerKind::Before => move_tr!("lists-dnd-drop-here").get(),
                    DropMarkerKind::End => move_tr!("lists-dnd-drop-at-end").get(),
                }}
            </span>
            <span class=move || drop_marker_line_class(is_active.get(), is_hovered.get())></span>
        </div>
    }
}

#[component]
pub fn ItemDragShell(
    dnd_state: RwSignal<ItemDndState>,
    dragged_item: DraggedItem,
    children: Children,
) -> impl IntoView {
    let dragged_item_for_state = dragged_item;
    let is_dragged = Signal::derive(move || {
        dnd_state.with(|state| is_dragged_item(state, &dragged_item_for_state))
    });

    view! {
        <div class=move || drag_shell_class(is_dragged.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn ItemDragHandleButton(
    dnd_state: RwSignal<ItemDndState>,
    dragged_item: DraggedItem,
    label: DragHandleLabel,
    #[prop(default = "")] extra_class: &'static str,
) -> impl IntoView {
    let dragged_item_for_state = dragged_item.clone();
    let dragged_item_for_dragstart = dragged_item.clone();
    let is_dragged = Signal::derive(move || {
        dnd_state.with(|state| is_dragged_item(state, &dragged_item_for_state))
    });

    view! {
        <button
            type="button"
            class=move || {
                let base_class = drag_handle_class(is_dragged.get());
                if extra_class.is_empty() {
                    base_class.to_string()
                } else {
                    format!("{base_class} {extra_class}")
                }
            }
            draggable="true"
            aria-label=move || match label {
                DragHandleLabel::Reorder => move_tr!("lists-dnd-reorder-aria").get(),
                DragHandleLabel::ReorderGroup => move_tr!("lists-dnd-reorder-group-aria").get(),
            }
            title=move || match label {
                DragHandleLabel::Reorder => move_tr!("lists-dnd-reorder-aria").get(),
                DragHandleLabel::ReorderGroup => move_tr!("lists-dnd-reorder-group-aria").get(),
            }
            on:dragstart=move |event: web_sys::DragEvent| {
                configure_drag_start(&event, &dragged_item_for_dragstart.item_id);
                dnd_state.update(|state| begin_item_drag(state, dragged_item_for_dragstart.clone()));
            }
            on:dragend=move |_| dnd_state.update(clear_item_dnd_state)
        >
            <DragGrip />
        </button>
    }
}

#[component]
pub fn ItemDragSurface(
    dnd_state: RwSignal<ItemDndState>,
    dragged_item: DraggedItem,
    hover_target: ItemDropTarget,
    children: Children,
) -> impl IntoView {
    let dragged_item_for_state = dragged_item;
    let hover_target_for_state = hover_target;
    let is_dragged = Signal::derive(move || {
        dnd_state.with(|state| is_dragged_item(state, &dragged_item_for_state))
    });
    let is_hovered = Signal::derive(move || {
        dnd_state.with(|state| is_hovered_item_target(state, &hover_target_for_state))
    });

    view! {
        <div class=move || drag_surface_class(is_dragged.get(), is_hovered.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn DragGrip() -> impl IntoView {
    view! {
        <span class="grid grid-cols-2 gap-1" aria-hidden="true">
            <span class="h-1 w-1 rounded-full bg-current"></span>
            <span class="h-1 w-1 rounded-full bg-current"></span>
            <span class="h-1 w-1 rounded-full bg-current"></span>
            <span class="h-1 w-1 rounded-full bg-current"></span>
            <span class="h-1 w-1 rounded-full bg-current"></span>
            <span class="h-1 w-1 rounded-full bg-current"></span>
        </span>
    }
}
