use leptos::prelude::*;

use crate::components::confirm_delete_modal::ConfirmDeleteModal;

#[component]
pub fn ListHeader(
    list_name: String,
    list_id: String,
    item_count: usize,
    on_delete_confirmed: Callback<()>,
    #[prop(optional)] on_archive: Option<Callback<()>>,
    #[prop(optional)] on_reset: Option<Callback<()>>,
) -> impl IntoView {
    let show_delete = RwSignal::new(false);

    view! {
        <div class="flex items-center justify-between mb-4">
            <h2 class="text-2xl font-bold">"Lista"</h2>
            <div class="flex gap-1">
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
