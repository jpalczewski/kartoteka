use leptos::prelude::*;

/// Click-to-edit description component. Shows text or placeholder normally,
/// textarea on click. Saves on blur, cancels on Escape.
#[component]
pub fn EditableDescription(
    value: Option<String>,
    on_save: Callback<Option<String>>,
    #[prop(default = "Dodaj opis...".to_string())] placeholder: String,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let edit_value = RwSignal::new(value.clone().unwrap_or_default());
    let original = RwSignal::new(value.clone());

    move || {
        if editing.get() {
            view! {
                <textarea
                    class="textarea textarea-bordered w-full text-sm mb-4"
                    rows=2
                    placeholder=placeholder.clone()
                    prop:value=move || edit_value.get()
                    on:input=move |ev| edit_value.set(event_target_value(&ev))
                    on:blur=move |_| {
                        let new_val = edit_value.get_untracked();
                        editing.set(false);
                        let old = original.get_untracked().unwrap_or_default();
                        if new_val != old {
                            let new_opt = if new_val.is_empty() { None } else { Some(new_val) };
                            original.set(new_opt.clone());
                            on_save.run(new_opt);
                        }
                    }
                    on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                        if ev.key() == "Escape" {
                            editing.set(false);
                        }
                    }
                    node_ref={
                        let node = NodeRef::<leptos::html::Textarea>::new();
                        leptos::task::spawn_local(async move {
                            if let Some(el) = node.get() {
                                let _ = el.focus();
                            }
                        });
                        node
                    }
                />
            }
            .into_any()
        } else {
            let desc = original.get();
            match desc {
                Some(d) if !d.is_empty() => view! {
                    <p
                        class="text-sm text-base-content/60 mb-4 cursor-pointer hover:text-base-content transition-colors"
                        title="Kliknij aby edytować opis"
                        on:click=move |_| {
                            edit_value.set(original.get_untracked().unwrap_or_default());
                            editing.set(true);
                        }
                    >
                        {d}
                    </p>
                }
                .into_any(),
                _ => view! {
                    <p
                        class="text-sm text-base-content/30 mb-4 cursor-pointer hover:text-base-content/50 transition-colors italic"
                        on:click=move |_| {
                            edit_value.set(String::new());
                            editing.set(true);
                        }
                    >
                        {placeholder.clone()}
                    </p>
                }
                .into_any(),
            }
        }
    }
}
