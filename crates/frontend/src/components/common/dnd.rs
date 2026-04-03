use leptos::prelude::*;

pub const END_DROP_TARGET_ID: &str = "__dnd_end__";

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
