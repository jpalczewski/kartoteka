use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::DateItem;

use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::get_all_items;
use crate::state::invalidation::use_invalidation;
use crate::state::item_actions::use_item_actions;
use crate::utils::group_by_list;

#[component]
pub fn AllPage() -> impl IntoView {
    let invalidation = use_invalidation();
    let (show_completed, set_show_completed) = signal(true);

    let all_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Lists]),
        |_| get_all_items(),
    );

    let item_actions = use_item_actions();
    let on_toggle = move |item_id: String| item_actions.toggle.run(item_id);

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || all_res.get().map(|result| match result {
                    Err(e) => {
                        let detail = e.to_string();
                        view! {
                            <p class="text-error">
                                {move_tr!("all-loading-error", { "detail" => detail.clone() })}
                            </p>
                        }.into_any()
                    },
                    Ok(items) => {
                        let total = items.len();
                        let completed_count = items.iter().filter(|di| di.item.completed).count();
                        let progress = format!("{completed_count}/{total}");
                        let has_items = !items.is_empty();

                        view! {
                            <div>
                                <div class="flex items-center justify-between mb-6">
                                    <h1 class="text-2xl font-bold">{move_tr!("all-title")}</h1>
                                </div>

                                {move || {
                                    let progress = progress.clone();
                                    let visible_items: Vec<DateItem> = items
                                        .iter()
                                        .filter(|di| show_completed.get() || !di.item.completed)
                                        .cloned()
                                        .collect();

                                    if !has_items {
                                        view! {
                                            <p class="text-center text-base-content/50 py-12">
                                                {move_tr!("all-empty")}
                                            </p>
                                        }.into_any()
                                    } else {
                                        let all_hidden = visible_items.is_empty();
                                        let groups = group_by_list(visible_items);

                                        view! {
                                            <div>
                                                <div class="flex items-center justify-between mb-2">
                                                    <span class="text-sm text-base-content/60" data-testid="all-completion-count">
                                                        {move_tr!("all-completed-count", { "progress" => progress.clone() })}
                                                    </span>
                                                    <label class="flex items-center gap-2 cursor-pointer select-none">
                                                        <span class="text-xs text-base-content/50">{move_tr!("all-hide-completed")}</span>
                                                        <input
                                                            type="checkbox"
                                                            class="toggle toggle-xs"
                                                            data-testid="hide-completed-toggle"
                                                            prop:checked=move || !show_completed.get()
                                                            on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                                        />
                                                    </label>
                                                </div>

                                                {if all_hidden {
                                                    view! {
                                                        <div class="text-center text-base-content/50 py-4">
                                                            {move_tr!("all-all-completed")}
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    render_groups(groups, on_toggle).into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

/// Notes and log lists have no completion concept, so their items must not render a checkbox.
fn shows_checkbox(list_type: &str) -> bool {
    !matches!(list_type, "notes" | "log")
}

fn render_groups(
    groups: Vec<(String, String, Vec<DateItem>)>,
    on_toggle: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    groups
        .into_iter()
        .map(|(list_id, list_name, items)| {
            view! {
                <div class="mb-4">
                    <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                            {list_name}
                        </A>
                    </h4>
                    <div class="flex flex-col gap-1">
                        {items.into_iter().map(|date_item| {
                            let item_id = date_item.item.id.clone();
                            let href = format!("/lists/{}/items/{}", date_item.item.list_id, date_item.item.id);
                            let completed = date_item.item.completed;
                            let title = date_item.item.title.clone();
                            let deadline = date_item.item.deadline
                                .as_ref()
                                .map(|d| d.to_string());
                            let checkbox = shows_checkbox(&date_item.list_type);
                            let description = if checkbox {
                                None
                            } else {
                                date_item.item.description.clone().filter(|d| !d.trim().is_empty())
                            };
                            let title_class = if completed {
                                "flex-1 text-sm text-base-content/50 line-through"
                            } else {
                                "flex-1 text-sm text-base-content hover:text-primary"
                            };
                            let desc_title_class = if completed {
                                "text-sm text-base-content/50 line-through"
                            } else {
                                "text-sm text-base-content hover:text-primary"
                            };

                            match description {
                                Some(description) => view! {
                                    <div class="flex items-start gap-3 p-2 bg-base-200 rounded-lg">
                                        <div class="flex-1 min-w-0">
                                            <A href=href attr:class=desc_title_class>
                                                {title}
                                            </A>
                                            <p
                                                class="text-xs text-base-content/60 line-clamp-2"
                                                data-testid="all-item-description"
                                            >
                                                {description}
                                            </p>
                                        </div>
                                        {deadline.map(|d| view! {
                                            <span class="text-xs text-error/70">{d}</span>
                                        })}
                                    </div>
                                }.into_any(),
                                None => view! {
                                    <div class="flex items-center gap-3 p-2 bg-base-200 rounded-lg">
                                        {checkbox.then(|| view! {
                                            <input
                                                type="checkbox"
                                                class="checkbox checkbox-sm checkbox-primary"
                                                data-testid="all-item-toggle"
                                                checked=completed
                                                on:change=move |_| on_toggle(item_id.clone())
                                            />
                                        })}
                                        <A href=href attr:class=title_class>
                                            {title}
                                        </A>
                                        {deadline.map(|d| view! {
                                            <span class="text-xs text-error/70">{d}</span>
                                        })}
                                    </div>
                                }.into_any(),
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })
        .collect_view()
}

#[cfg(test)]
mod tests {
    use super::shows_checkbox;

    #[test]
    fn checkbox_only_for_completable_list_types() {
        assert!(shows_checkbox("checklist"));
        assert!(shows_checkbox("shopping"));
        assert!(shows_checkbox("schedule"));
        assert!(!shows_checkbox("log"));
        assert!(!shows_checkbox("notes"));
    }
}
