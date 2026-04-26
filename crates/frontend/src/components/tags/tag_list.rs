use kartoteka_shared::types::Tag;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::tags::tag_badge::TagBadge;
use crate::components::tags::tag_tree::build_breadcrumb;

/// Renders a row of tag badges for a list or item.
/// If `on_toggle` is provided, each badge shows an X button on hover that removes the tag
/// (with a confirmation modal). The badge itself navigates to the tag detail page.
/// A "+" dropdown lets the user assign any unassigned tag.
#[component]
pub fn TagList(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    #[prop(optional)] on_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let selected_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| selected_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    let unassigned_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| !selected_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    if selected_tags.is_empty() && on_toggle.is_none() {
        return view! {}.into_any();
    }

    let pending_remove: RwSignal<Option<String>> = RwSignal::new(None);
    let navigate = use_navigate();
    let removable = on_toggle.is_some();

    let tag_labels: std::collections::HashMap<String, String> = selected_tags
        .iter()
        .map(|tag| {
            let label = build_breadcrumb(&all_tags, &tag.id)
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(" / ");
            (tag.id.clone(), label)
        })
        .collect();

    view! {
        <div class="flex flex-wrap items-center gap-1">
            {selected_tags.into_iter().map(|tag| {
                let tag_id = tag.id.clone();
                let label = tag_labels.get(&tag.id).cloned().unwrap_or_default();
                let nav = navigate.clone();
                let tid_nav = tag_id.clone();
                let badge = view! {
                    <TagBadge
                        tag=tag
                        label=label
                        on_click=Callback::new(move |_: String| {
                            nav(&format!("/tags/{}", tid_nav), Default::default());
                        })
                    />
                };

                if removable {
                    let tid_remove = tag_id.clone();
                    let hovered = RwSignal::new(false);
                    view! {
                        <div
                            class="relative"
                            on:mouseenter=move |_| hovered.set(true)
                            on:mouseleave=move |_| hovered.set(false)
                        >
                            {badge}
                            <button
                                type="button"
                                class=move || format!(
                                    "absolute -top-1 -right-1 w-4 h-4 rounded-full bg-base-300 hover:bg-error hover:text-error-content text-xs flex items-center justify-center z-10 leading-none transition-opacity {}",
                                    if hovered.get() { "opacity-100" } else { "opacity-0 pointer-events-none" }
                                )
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    pending_remove.set(Some(tid_remove.clone()));
                                }
                            >
                                "×"
                            </button>
                        </div>
                    }.into_any()
                } else {
                    badge.into_any()
                }
            }).collect::<Vec<_>>()}

            {on_toggle.map(|cb| {
                if unassigned_tags.is_empty() {
                    return view! {}.into_any();
                }
                view! {
                    <div class="dropdown" on:click=move |ev| ev.stop_propagation()>
                        <div
                            tabindex="0"
                            role="button"
                            class="btn btn-ghost btn-xs"
                            data-testid="tag-add-btn"
                        >
                            "+"
                        </div>
                        <ul
                            tabindex="0"
                            class="dropdown-content menu bg-base-100 rounded-box z-50 w-40 p-1 shadow-lg border border-base-300"
                        >
                            {unassigned_tags.into_iter().map(|tag| {
                                let tid = tag.id.clone();
                                let color = tag.color.clone().unwrap_or_else(|| "#6366f1".to_string());
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            class="flex items-center gap-2 text-sm"
                                            data-testid="tag-dropdown-option"
                                            on:click=move |_| cb.run(tid.clone())
                                        >
                                            <span
                                                class="w-3 h-3 rounded-full inline-block shrink-0"
                                                style=format!("background:{color}")
                                            ></span>
                                            {tag.name.clone()}
                                        </button>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            })}
        </div>

        <ConfirmModal
            open=Signal::derive(move || pending_remove.get().is_some())
            title="Odepnij tag".to_string()
            message="Czy na pewno chcesz odpiąć ten tag?".to_string()
            confirm_label="Odepnij".to_string()
            variant=ConfirmVariant::Warning
            on_close=Callback::new(move |_| pending_remove.set(None))
            on_confirm=Callback::new(move |_| {
                if let Some((tid, cb)) = pending_remove.get().zip(on_toggle) {
                    pending_remove.set(None);
                    cb.run(tid);
                }
            })
        />
    }
    .into_any()
}
