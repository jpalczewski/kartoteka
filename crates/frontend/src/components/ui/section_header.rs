use leptos::prelude::*;

use super::icon::{Icon, IconName};

/// Section heading with an optional icon, count and a right-aligned actions slot.
///
/// Used for the dashboard sections and the sublist block so all of them share
/// the same title/count/action layout.
#[component]
pub fn SectionHeader(
    #[prop(into)] title: Signal<String>,
    /// Optional lucide icon before the title.
    #[prop(optional, into)]
    icon: Option<IconName>,
    /// Optional count rendered as a muted pill after the title.
    #[prop(optional)]
    count: Option<usize>,
    /// Extra classes for the wrapping row.
    #[prop(default = String::new(), into)]
    class: String,
    /// Actions rendered on the right (buttons, menus, …).
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class=format!("flex items-center gap-2 {class}")>
            {icon.map(|icon| view! { <Icon name=icon size=14 class="text-base-content/60" /> })}
            <h2 class="text-sm font-semibold uppercase tracking-wide text-base-content/60">
                {title}
            </h2>
            {count.map(|count| {
                view! {
                    <span class="badge badge-sm badge-ghost" data-testid="section-header-count">
                        {count}
                    </span>
                }
            })}
            <div class="ml-auto flex items-center gap-1">{children.map(|children| children())}</div>
        </div>
    }
}
