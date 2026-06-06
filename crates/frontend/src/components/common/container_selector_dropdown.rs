use kartoteka_shared::types::ContainerOption;
use leptos::prelude::*;

/// Floating dropdown list for selecting a move target container.
///
/// Caller controls visibility via `open` signal and provides pre-fetched `options`.
/// The trigger button and "Detach" button live inline on each page — they differ per context.
#[component]
pub fn ContainerSelectorDropdown(
    open: RwSignal<bool>,
    options: Vec<ContainerOption>,
    on_select: Callback<String>,
) -> impl IntoView {
    let opts = StoredValue::new(options);

    view! {
        <div
            class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-56 max-h-64 overflow-y-auto z-50 p-2 shadow-lg"
            style:display=move || if open.get() { "block" } else { "none" }
        >
            {move || opts.get_value().into_iter().map(|opt| {
                let cid = opt.id.clone();
                view! {
                    <button
                        type="button"
                        class="flex items-center px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300 w-full text-left"
                        on:click=move |_| {
                            open.set(false);
                            on_select.run(cid.clone());
                        }
                    >
                        {opt.path_label.clone()}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
