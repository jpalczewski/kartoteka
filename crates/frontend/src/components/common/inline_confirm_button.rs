use leptos::prelude::*;

/// Inline button that requires a second click to confirm.
/// First click shows confirm_label for `timeout_ms`, then reverts.
/// Distinct from `ConfirmDeleteModal` which is a modal dialog.
#[component]
pub fn InlineConfirmButton(
    /// Callback when confirmed (second click)
    on_confirm: Callback<()>,
    /// Label shown normally
    #[prop(default = "\u{2715}".to_string())]
    label: String,
    /// Label shown during confirm state
    #[prop(default = "Na pewno?".to_string())]
    confirm_label: String,
    /// CSS class for normal state
    #[prop(default = "btn btn-ghost btn-sm btn-square opacity-60 hover:opacity-100".to_string())]
    class: String,
    /// CSS class for confirm state
    #[prop(default = "btn btn-error btn-sm".to_string())]
    confirm_class: String,
    /// Timeout in ms before reverting to normal state
    #[prop(default = 2500)]
    timeout_ms: u32,
) -> impl IntoView {
    let confirming = RwSignal::new(false);

    view! {
        <button
            type="button"
            class=move || if confirming.get() { confirm_class.clone() } else { class.clone() }
            on:click=move |_| {
                if confirming.get() {
                    on_confirm.run(());
                    confirming.set(false);
                } else {
                    confirming.set(true);
                    set_timeout(
                        move || confirming.set(false),
                        std::time::Duration::from_millis(timeout_ms.into()),
                    );
                }
            }
        >
            {move || if confirming.get() { confirm_label.clone() } else { label.clone() }}
        </button>
    }
}
