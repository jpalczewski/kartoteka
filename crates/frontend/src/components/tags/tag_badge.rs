use kartoteka_shared::types::Tag;
use kartoteka_shared::{DEFAULT_TAG_COLOR, readable_text_color};
use leptos::prelude::*;

use crate::components::ui::EntityIcon;

/// A tag rendered as a coloured pill.
///
/// The background is the tag colour, the label colour is picked by
/// [`readable_text_color`] so light colours (yellow, lime, …) get dark text
/// instead of the unreadable white that used to be hardcoded. Tags with an icon
/// show it before the name. The `tag-badge` class stays on the root: e2e specs
/// and the list/item tag rows select badges by it.
#[component]
pub fn TagBadge(
    tag: Tag,
    #[prop(optional)] on_click: Option<Callback<String>>,
    #[prop(default = false)] active: bool,
    /// Override display text (e.g. full hierarchical path "Parent / Child").
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView {
    let color = tag
        .color
        .clone()
        .unwrap_or_else(|| DEFAULT_TAG_COLOR.to_string());
    let style = format!(
        "background: {color}; color: {};",
        readable_text_color(&color)
    );
    let tag_id = tag.id.clone();
    let display = label.unwrap_or(tag.name.clone());
    let icon = tag.icon.clone();

    view! {
        <span
            class=move || {
                if active {
                    "tag-badge badge gap-1 cursor-pointer ring-2 ring-offset-1 ring-base-content"
                } else {
                    "tag-badge badge gap-1 cursor-pointer"
                }
            }
            style=style
            on:click=move |ev| {
                if let Some(cb) = on_click.clone() {
                    ev.stop_propagation();
                    cb.run(tag_id.clone());
                }
            }
        >
            {icon.map(|raw| view! {
                <EntityIcon raw=Some(raw) fallback="tag" size=12 />
            })}
            {display}
        </span>
    }
}
