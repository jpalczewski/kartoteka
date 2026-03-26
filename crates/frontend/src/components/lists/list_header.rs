use kartoteka_shared::{FEATURE_DUE_DATE, FEATURE_QUANTITY, ListFeature};
use leptos::prelude::*;

use crate::components::confirm_delete_modal::ConfirmDeleteModal;
use crate::components::editable_title::EditableTitle;

#[component]
pub fn ListHeader(
    list_name: String,
    list_id: String,
    item_count: usize,
    on_delete_confirmed: Callback<()>,
    #[prop(optional)] on_archive: Option<Callback<()>>,
    #[prop(optional)] on_reset: Option<Callback<()>>,
    #[prop(optional)] on_rename: Option<Callback<String>>,
    #[prop(default = vec![])] features: Vec<ListFeature>,
    #[prop(optional)] on_feature_toggle: Option<Callback<(String, bool)>>,
) -> impl IntoView {
    let show_delete = RwSignal::new(false);
    let show_settings = RwSignal::new(false);
    let has_quantity = features.iter().any(|f| f.name == FEATURE_QUANTITY);
    let has_due_date = features.iter().any(|f| f.name == FEATURE_DUE_DATE);

    view! {
        <div class="flex items-center justify-between mb-4">
            {if let Some(on_rename) = on_rename {
                view! {
                    <EditableTitle value=list_name.clone() on_save=on_rename />
                }.into_any()
            } else {
                view! { <h2 class="text-2xl font-bold">{list_name.clone()}</h2> }.into_any()
            }}
            <div class="flex gap-1">
                {on_feature_toggle.map(|_| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| show_settings.update(|v| *v = !*v)
                    >
                        "\u{2699}\u{FE0F}"
                    </button>
                })}
                {on_reset.map(|cb| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| cb.run(())
                    >
                        "\u{1F504} Reset"
                    </button>
                })}
                {on_archive.map(|cb| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                        on:click=move |_| cb.run(())
                    >
                        "\u{1F4E6} Archiwizuj"
                    </button>
                })}
                <button
                    type="button"
                    class="btn btn-ghost btn-sm opacity-60 hover:opacity-100"
                    on:click=move |_| show_delete.set(true)
                >
                    "\u{1F5D1} Usu\u{0144} list\u{0119}"
                </button>
            </div>
        </div>

        // Feature settings panel
        {move || {
            if show_settings.get() {
                on_feature_toggle.map(|on_toggle| view! {
                    <div class="bg-base-200 rounded-lg p-3 mb-4 flex items-center gap-4">
                        <span class="text-sm font-semibold">"Funkcje:"</span>
                        <label class="label cursor-pointer gap-2">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-sm"
                                prop:checked=has_quantity
                                on:change=move |ev| {
                                    on_toggle.run((FEATURE_QUANTITY.to_string(), event_target_checked(&ev)));
                                }
                            />
                            <span class="label-text">"Ilości"</span>
                        </label>
                        <label class="label cursor-pointer gap-2">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-sm"
                                prop:checked=has_due_date
                                on:change=move |ev| {
                                    on_toggle.run((FEATURE_DUE_DATE.to_string(), event_target_checked(&ev)));
                                }
                            />
                            <span class="label-text">"Terminy"</span>
                        </label>
                    </div>
                })
            } else {
                None
            }
        }}

        // Delete confirmation modal
        {move || {
            if show_delete.get() {
                let lid = list_id.clone();
                let lname = list_name.clone();
                Some(view! {
                    <ConfirmDeleteModal
                        list_id=lid
                        list_name=lname
                        item_count=item_count
                        on_confirm=Callback::new(move |_| {
                            on_delete_confirmed.run(());
                        })
                        on_cancel=Callback::new(move |_| show_delete.set(false))
                    />
                })
            } else {
                None
            }
        }}
    }
}
