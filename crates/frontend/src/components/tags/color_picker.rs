use kartoteka_shared::{DEFAULT_TAG_COLOR, TAG_PALETTE};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::ui::IconButton;

/// Index into [`TAG_PALETTE`] for a "random" pick.
///
/// Only ever called from a browser event handler, so SSR never executes it; the
/// SSR stub keeps the module compiling for the server.
#[cfg(target_arch = "wasm32")]
fn random_palette_color() -> String {
    let index = (web_sys::js_sys::Math::random() * TAG_PALETTE.len() as f64) as usize;
    TAG_PALETTE
        .get(index)
        .copied()
        .unwrap_or(DEFAULT_TAG_COLOR)
        .to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn random_palette_color() -> String {
    DEFAULT_TAG_COLOR.to_string()
}

/// Palette of tag colours plus a native colour input for anything custom.
///
/// Picking a swatch, a custom colour or the random button all go through
/// `on_change`, so the parent stays the single owner of the value. The random
/// button picks from [`TAG_PALETTE`] instead of a free-form hex value, which
/// used to produce unreadable colours.
#[component]
pub fn TagColorPicker(
    #[prop(into)] value: Signal<String>,
    on_change: Callback<String>,
    /// `data-testid` of the wrapping element.
    #[prop(optional_no_strip)]
    testid: Option<&'static str>,
) -> impl IntoView {
    let on_change_swatch = on_change.clone();
    let on_change_custom = on_change.clone();
    let on_change_random = on_change.clone();
    let current = value;

    view! {
        <div class="flex flex-wrap items-center gap-1" data-testid=testid>
            {TAG_PALETTE
                .iter()
                .map(|color| {
                    let color = (*color).to_string();
                    let color_for_class = color.clone();
                    let color_for_click = color.clone();
                    view! {
                        <button
                            type="button"
                            class=move || {
                                let selected = current.get().eq_ignore_ascii_case(&color_for_class);
                                if selected {
                                    "w-5 h-5 rounded-full border border-base-300 ring-2 ring-offset-1 ring-offset-base-100 ring-primary transition-transform hover:scale-110"
                                } else {
                                    "w-5 h-5 rounded-full border border-base-300 opacity-70 transition-transform hover:scale-110 hover:opacity-100"
                                }
                            }
                            style=format!("background: {color};")
                            aria-label=color.clone()
                            title=color.clone()
                            data-testid="tag-color-swatch"
                            on:click=move |_| on_change_swatch.run(color_for_click.clone())
                        ></button>
                    }
                })
                .collect_view()}

            // Custom colour: the native picker stays as the escape hatch for any hex value.
            <input
                type="color"
                class="w-6 h-6 rounded cursor-pointer border border-base-300 bg-transparent"
                title=move_tr!("tags-color-aria")
                data-testid="tag-color-input"
                prop:value=move || current.get()
                on:change=move |ev| on_change_custom.run(event_target_value(&ev))
            />

            <IconButton
                icon="shuffle"
                label=move_tr!("tags-color-random")
                size=crate::components::ui::IconButtonSize::Xs
                testid=Some("tag-color-random-btn")
                on_click=Callback::new(move |_| on_change_random.run(random_palette_color()))
            />
        </div>
    }
}
