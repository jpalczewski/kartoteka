use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use super::icon::EntityIcon;
use super::icon_picker::IconPicker;
use crate::components::tags::color_picker::TagColorPicker;

/// Modal for choosing an icon and an accent colour.
///
/// Presentation-only: it edits a draft copy of `icon` / `color` and hands the
/// result to `on_save` as `(icon, color)`; `None` means "back to the default"
/// (default icon / neutral colour). The draft is reset from the props every time
/// the dialog opens, so cancelling discards the changes.
#[component]
pub fn AppearanceDialog(
    open: RwSignal<bool>,
    /// Stored icon value of the entity (`lucide:<name>`, `emoji:<e>`, bare emoji).
    icon: Option<String>,
    /// Stored accent colour (`#rrggbb`) of the entity.
    color: Option<String>,
    /// Lucide name used when the icon is cleared.
    default_icon: &'static str,
    /// Called with the chosen `(icon, color)` when the user saves.
    on_save: Callback<(Option<String>, Option<String>)>,
) -> impl IntoView {
    let draft_icon = RwSignal::new(icon.clone());
    let draft_color = RwSignal::new(color.clone());

    Effect::new(move |_| {
        if open.get() {
            draft_icon.set(icon.clone());
            draft_color.set(color.clone());
        }
    });

    let close = move || open.set(false);

    view! {
        <Show when=move || open.get()>
            <dialog
                class="modal"
                open=true
                data-testid="appearance-dialog"
                on:keydown=move |ev: KeyboardEvent| {
                    if ev.key() == "Escape" {
                        close();
                    }
                }
            >
                <div class="modal-box max-w-md flex flex-col gap-4">
                    <div class="flex items-center gap-3">
                        <span
                            class="inline-flex size-12 items-center justify-center rounded-box bg-base-200 border border-base-300 text-2xl"
                            data-testid="appearance-preview"
                        >
                            {move || {
                                let color = draft_color.get();
                                view! {
                                    <EntityIcon
                                        raw=draft_icon.get()
                                        fallback=default_icon
                                        color=color.clone()
                                        size=26
                                    />
                                }
                            }}
                        </span>
                        <h3 class="font-bold text-lg">{move_tr!("appearance-title")}</h3>
                    </div>

                    <IconPicker
                        value=draft_icon
                        on_change=Callback::new(move |value: Option<String>| draft_icon.set(value))
                        default_icon=default_icon
                    />

                    <div class="flex flex-col gap-2">
                        <span class="text-sm font-semibold">{move_tr!("appearance-color")}</span>
                        <div class="flex flex-wrap items-center gap-2">
                            <button
                                type="button"
                                class=move || {
                                    if draft_color.get().is_none() {
                                        "btn btn-xs btn-primary btn-soft"
                                    } else {
                                        "btn btn-xs btn-ghost"
                                    }
                                }
                                aria-pressed=move || draft_color.get().is_none().to_string()
                                data-testid="appearance-color-none"
                                on:click=move |_| draft_color.set(None)
                            >
                                {move_tr!("appearance-color-none")}
                            </button>
                            <TagColorPicker
                                value=Signal::derive(move || draft_color.get().unwrap_or_default())
                                on_change=Callback::new(move |value: String| draft_color.set(Some(value)))
                                testid=Some("appearance-color-picker")
                            />
                        </div>
                    </div>

                    <div class="modal-action mt-0">
                        <button
                            type="button"
                            class="btn btn-ghost"
                            data-testid="appearance-cancel"
                            on:click=move |_| close()
                        >
                            {move_tr!("common-cancel")}
                        </button>
                        <button
                            type="button"
                            class="btn btn-primary"
                            data-testid="appearance-save"
                            on:click=move |_| {
                                on_save.run((draft_icon.get_untracked(), draft_color.get_untracked()));
                                close();
                            }
                        >
                            {move_tr!("common-save")}
                        </button>
                    </div>
                </div>
                <div class="modal-backdrop">
                    <button type="button" tabindex="-1" aria-label="close" on:click=move |_| close()></button>
                </div>
            </dialog>
        </Show>
    }
}
