use crate::components::common::date_utils::DateBadge;
use leptos::prelude::*;

/// Renders clickable date badge chips that toggle an editing_date signal.
/// Used by ItemRow and DateItemRow.
#[component]
pub fn DateBadgeChips(
    badges: Vec<DateBadge>,
    editing_date: RwSignal<Option<String>>,
    /// Show ghost chips for unset date types
    #[prop(default = false)]
    ghost_start: bool,
    #[prop(default = false)] ghost_deadline: bool,
    #[prop(default = false)] ghost_hard: bool,
) -> impl IntoView {
    let has_ghosts = ghost_start || ghost_deadline || ghost_hard;
    if badges.is_empty() && !has_ghosts {
        return view! {}.into_any();
    }

    view! {
        <div class="flex gap-1 flex-wrap shrink-0">
            {badges.into_iter().map(|b| {
                let dt = b.date_type.to_string();
                view! {
                    <button type="button" class=format!("{} cursor-pointer", b.css)
                        on:click=move |_| {
                            let current = editing_date.get();
                            if current.as_deref() == Some(dt.as_str()) {
                                editing_date.set(None);
                            } else {
                                editing_date.set(Some(dt.clone()));
                            }
                        }
                    >{b.label}</button>
                }
            }).collect::<Vec<_>>()}
            {if ghost_start {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("start".into()))
                    >"+\u{1F4C5}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
            {if ghost_deadline {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("deadline".into()))
                    >"+\u{23F0}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
            {if ghost_hard {
                view! {
                    <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                        on:click=move |_| editing_date.set(Some("hard_deadline".into()))
                    >"+\u{1F6A8}"</button>
                }.into_any()
            } else { view! {}.into_any() }}
        </div>
    }.into_any()
}
