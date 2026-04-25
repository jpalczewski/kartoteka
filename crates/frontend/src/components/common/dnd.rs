//! Reusable drag-and-drop UI primitives: drag handles, shells, surfaces,
//! reorder markers, nest surfaces and a detach dropzone.
//!
//! All components operate against a caller-provided `RwSignal<DndState>` or
//! `RwSignal<ItemDndState>` — pages own state; components are stateless.

use leptos::prelude::*;
use web_sys::DragEvent;

use crate::state::dnd::{
    DndState, DraggedItem, DropTarget, EntityKind, ItemDndState, ItemDropTarget, begin_drag,
    clear_dnd_state, clear_item_dnd_state, is_dragged_id, is_dragged_item, is_hovered_item_target,
    is_hovered_target, set_hovered_item_target, set_hovered_target,
};

// ── Style helpers ─────────────────────────────────────────────────────────

pub fn drag_shell_class(is_dragged: bool) -> &'static str {
    if is_dragged {
        "flex items-stretch gap-2 opacity-50 scale-[0.985] transition-all duration-150"
    } else {
        "flex items-stretch gap-2 transition-all duration-150"
    }
}

pub fn drag_surface_class(is_dragged: bool, is_nest_hovered: bool) -> &'static str {
    if is_dragged {
        "flex-1 rounded-lg ring-2 ring-primary/20 opacity-70 transition-all duration-150"
    } else if is_nest_hovered {
        "flex-1 rounded-lg ring-2 ring-primary bg-primary/5 transition-all duration-150"
    } else {
        "flex-1 rounded-lg transition-all duration-150"
    }
}

pub fn drag_handle_class(is_dragged: bool) -> &'static str {
    if is_dragged {
        "inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-primary/35 bg-primary/15 text-primary shadow-sm cursor-grabbing"
    } else {
        "inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-base-300 bg-base-100 text-base-content/40 shadow-sm cursor-grab transition-all duration-100 hover:border-primary/35 hover:bg-primary/10 hover:text-primary"
    }
}

pub fn reorder_marker_class(is_active: bool, is_hovered: bool) -> &'static str {
    if is_active && is_hovered {
        "flex h-10 items-center gap-2 overflow-hidden rounded-lg border border-primary/55 bg-primary/10 px-3 transition-all duration-100"
    } else if is_active {
        "flex h-7 items-center gap-2 overflow-hidden rounded-lg border border-dashed border-primary/30 px-3 transition-all duration-100 hover:border-primary/60 hover:bg-primary/5"
    } else {
        "pointer-events-none flex h-0 items-center gap-2 overflow-hidden rounded-lg border border-transparent px-3 opacity-0 transition-all duration-100"
    }
}

pub fn detach_zone_class(is_active: bool, is_hovered: bool) -> &'static str {
    if is_active && is_hovered {
        "mb-3 p-3 border-2 border-dashed border-primary bg-primary/10 rounded-lg text-center text-sm text-primary transition-all duration-100"
    } else if is_active {
        "mb-3 p-3 border-2 border-dashed border-base-300 rounded-lg text-center text-sm text-base-content/50 transition-all duration-100"
    } else {
        "pointer-events-none h-0 overflow-hidden opacity-0 transition-all duration-100"
    }
}

// ── Low-level DataTransfer helpers ────────────────────────────────────────

fn configure_drag_over(event: &DragEvent) {
    event.prevent_default();
    if let Some(dt) = event.data_transfer() {
        dt.set_drop_effect("move");
    }
}

fn configure_drag_start(event: &DragEvent, dragged_id: &str) {
    if let Some(dt) = event.data_transfer() {
        let _ = dt.set_data("text/plain", dragged_id);
        dt.set_effect_allowed("move");
    }
}

// ── Grip icon ─────────────────────────────────────────────────────────────

#[component]
pub fn DragGrip() -> impl IntoView {
    view! {
        <span class="grid grid-cols-2 gap-0.5" aria-hidden="true">
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
            <span class="h-0.5 w-0.5 rounded-full bg-current"></span>
        </span>
    }
}

// ── Generic (list / container) DnD components ────────────────────────────

#[component]
pub fn DragHandleButton(
    dnd_state: RwSignal<DndState>,
    kind: EntityKind,
    dragged_id: String,
    #[prop(default = "Przeciągnij, aby zmienić kolejność")] aria_label: &'static str,
) -> impl IntoView {
    let id_state = dragged_id.clone();
    let id_start = dragged_id;
    let is_dragged = Signal::derive(move || dnd_state.with(|s| is_dragged_id(s, &id_state)));

    view! {
        <button
            type="button"
            class=move || drag_handle_class(is_dragged.get())
            draggable="true"
            aria-label=aria_label
            title=aria_label
            data-testid="drag-handle"
            on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
            on:dragstart=move |ev: DragEvent| {
                configure_drag_start(&ev, &id_start);
                dnd_state.update(|s| begin_drag(s, kind, id_start.clone()));
            }
            on:dragend=move |_| dnd_state.update(clear_dnd_state)
        >
            <DragGrip />
        </button>
    }
}

#[component]
pub fn DragShell(
    dnd_state: RwSignal<DndState>,
    dragged_id: String,
    children: Children,
) -> impl IntoView {
    let id = dragged_id;
    let is_dragged = Signal::derive(move || dnd_state.with(|s| is_dragged_id(s, &id)));
    view! {
        <div class=move || drag_shell_class(is_dragged.get())>
            {children()}
        </div>
    }
}

/// Drag surface wraps the card body and accepts `DropTarget::Nest(id)` drops.
/// Reorder markers (`Before`/`End`) live as siblings, not inside the surface.
#[component]
pub fn DragSurface(
    dnd_state: RwSignal<DndState>,
    dragged_id: String,
    nest_target_id: String,
    /// Called with `DropTarget::Nest(nest_target_id)` when the user drops on
    /// the card body. Pages decide whether to accept.
    on_drop: Callback<DropTarget>,
    children: Children,
) -> impl IntoView {
    let id_dragged = dragged_id.clone();
    let nest_id_for_hover = nest_target_id.clone();
    let nest_id_for_over = nest_target_id.clone();
    let nest_id_for_drop = nest_target_id;
    let is_dragged = Signal::derive(move || dnd_state.with(|s| is_dragged_id(s, &id_dragged)));
    let is_hovered = Signal::derive(move || {
        dnd_state.with(|s| is_hovered_target(s, &DropTarget::Nest(nest_id_for_hover.clone())))
    });

    view! {
        <div
            class=move || drag_surface_class(is_dragged.get(), is_hovered.get())
            on:dragover=move |ev: DragEvent| {
                // Only accept nest when something is being dragged and it isn't self
                let active_and_not_self = dnd_state.with(|s| {
                    s.dragged_id().map(|d| d != nest_id_for_over).unwrap_or(false)
                });
                if active_and_not_self {
                    configure_drag_over(&ev);
                    let target = DropTarget::Nest(nest_id_for_over.clone());
                    dnd_state.update(|s| set_hovered_target(s, target));
                }
            }
            on:drop=move |ev: DragEvent| {
                ev.prevent_default();
                let target = DropTarget::Nest(nest_id_for_drop.clone());
                on_drop.run(target);
                dnd_state.update(clear_dnd_state);
            }
        >
            {children()}
        </div>
    }
}

#[component]
pub fn ReorderDropTarget(
    dnd_state: RwSignal<DndState>,
    target: DropTarget,
    on_drop: Callback<DropTarget>,
    #[prop(default = "Upuść tutaj")] label: &'static str,
) -> impl IntoView {
    let t_hover = target.clone();
    let t_over = target.clone();
    let t_drop = target.clone();
    let is_active = Signal::derive(move || dnd_state.with(|s| s.is_active()));
    let is_hovered = Signal::derive(move || dnd_state.with(|s| is_hovered_target(s, &t_hover)));

    view! {
        <div
            class=move || reorder_marker_class(is_active.get(), is_hovered.get())
            data-testid="reorder-marker"
            on:dragover=move |ev: DragEvent| {
                configure_drag_over(&ev);
                dnd_state.update(|s| set_hovered_target(s, t_over.clone()));
            }
            on:drop=move |ev: DragEvent| {
                ev.prevent_default();
                on_drop.run(t_drop.clone());
                dnd_state.update(clear_dnd_state);
            }
        >
            <span class="h-px flex-1 bg-gradient-to-r from-transparent via-primary/55 to-transparent"></span>
            <span class=move || if is_active.get() && is_hovered.get() {
                "shrink-0 text-[10px] font-bold uppercase tracking-[0.2em] text-primary"
            } else if is_active.get() {
                "shrink-0 text-[10px] font-semibold uppercase tracking-[0.18em] text-primary/60"
            } else {
                "hidden"
            }>{label}</span>
            <span class="h-px flex-1 bg-gradient-to-r from-transparent via-primary/55 to-transparent"></span>
        </div>
    }
}

/// A full-width dropzone shown above a section. Visible only while a drag is
/// active and the visibility signal returns true (e.g., payload is an eligible
/// child of the current page).
#[component]
pub fn DetachDropZone(
    dnd_state: RwSignal<DndState>,
    /// True when a drag is in progress AND the dragged entity is eligible to
    /// detach on this page (pages compute this, e.g. "payload.id is a direct
    /// child of current container").
    visible: Signal<bool>,
    on_drop: Callback<()>,
    #[prop(default = "Upuść tutaj, aby wyjąć do rodzica")] label: &'static str,
) -> impl IntoView {
    let is_hovered =
        Signal::derive(move || dnd_state.with(|s| is_hovered_target(s, &DropTarget::Detach)));

    view! {
        <div
            class=move || detach_zone_class(visible.get(), is_hovered.get())
            data-testid="detach-zone"
            on:dragover=move |ev: DragEvent| {
                if visible.get() {
                    configure_drag_over(&ev);
                    dnd_state.update(|s| set_hovered_target(s, DropTarget::Detach));
                }
            }
            on:drop=move |ev: DragEvent| {
                ev.prevent_default();
                if visible.get() {
                    on_drop.run(());
                    dnd_state.update(clear_dnd_state);
                }
            }
        >
            {label}
        </div>
    }
}

// ── Item DnD components ───────────────────────────────────────────────────

#[component]
pub fn ItemDragHandleButton(
    dnd_state: RwSignal<ItemDndState>,
    dragged_item: DraggedItem,
    #[prop(default = "Przeciągnij element")] aria_label: &'static str,
) -> impl IntoView {
    let item_state = dragged_item;
    let is_dragged = Signal::derive(move || dnd_state.with(|s| is_dragged_item(s, &item_state)));

    view! {
        <button
            type="button"
            class=move || drag_handle_class(is_dragged.get())
            aria-label=aria_label
            title=aria_label
            data-testid="item-drag-handle"
            on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
        >
            <DragGrip />
        </button>
    }
}

#[component]
pub fn ItemDragShell(
    dnd_state: RwSignal<ItemDndState>,
    dragged_item: DraggedItem,
    children: Children,
) -> impl IntoView {
    let it = dragged_item;
    let is_dragged = Signal::derive(move || dnd_state.with(|s| is_dragged_item(s, &it)));
    view! {
        <div class=move || drag_shell_class(is_dragged.get())>
            {children()}
        </div>
    }
}

#[component]
pub fn ItemDropTargetMarker(
    dnd_state: RwSignal<ItemDndState>,
    target: ItemDropTarget,
    on_drop: Callback<ItemDropTarget>,
    #[prop(default = "Upuść tutaj")] label: &'static str,
) -> impl IntoView {
    let t_hover = target.clone();
    let t_over = target.clone();
    let t_drop = target.clone();
    let is_active = Signal::derive(move || dnd_state.with(|s| s.is_active()));
    let is_hovered =
        Signal::derive(move || dnd_state.with(|s| is_hovered_item_target(s, &t_hover)));

    view! {
        <div
            class=move || reorder_marker_class(is_active.get(), is_hovered.get())
            data-testid="item-reorder-marker"
            on:dragover=move |ev: DragEvent| {
                configure_drag_over(&ev);
                dnd_state.update(|s| set_hovered_item_target(s, t_over.clone()));
            }
            on:drop=move |ev: DragEvent| {
                ev.prevent_default();
                on_drop.run(t_drop.clone());
                dnd_state.update(clear_item_dnd_state);
            }
        >
            <span class="h-px flex-1 bg-gradient-to-r from-transparent via-primary/55 to-transparent"></span>
            <span class=move || if is_active.get() && is_hovered.get() {
                "shrink-0 text-[10px] font-bold uppercase tracking-[0.2em] text-primary"
            } else if is_active.get() {
                "shrink-0 text-[10px] font-semibold uppercase tracking-[0.18em] text-primary/60"
            } else {
                "hidden"
            }>{label}</span>
            <span class="h-px flex-1 bg-gradient-to-r from-transparent via-primary/55 to-transparent"></span>
        </div>
    }
}
