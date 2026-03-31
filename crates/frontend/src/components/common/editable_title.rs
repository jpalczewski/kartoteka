use leptos::prelude::*;

/// Click-to-edit title component. Shows text normally, input on click.
/// Saves on blur or Enter, cancels on Escape.
#[component]
pub fn EditableTitle(
    value: String,
    on_save: Callback<String>,
    #[prop(default = "text-2xl font-bold".to_string())] class: String,
) -> impl IntoView {
    let (editing, set_editing) = signal(false);
    let edit_value = RwSignal::new(value.clone());
    let original = RwSignal::new(value.clone());

    view! {
        {move || {
            if editing.get() {
                view! {
                    <input
                        type="text"
                        class=format!("input input-bordered h-10 w-full {}", class)
                        prop:value=move || edit_value.get()
                        on:input=move |ev| edit_value.set(event_target_value(&ev))
                        on:blur=move |_| {
                            let new_val = edit_value.get_untracked();
                            set_editing.set(false);
                            if !new_val.is_empty() && new_val != original.get_untracked() {
                                original.set(new_val.clone());
                                on_save.run(new_val);
                            }
                        }
                        on:keydown=move |ev: leptos::ev::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                let new_val = edit_value.get_untracked();
                                set_editing.set(false);
                                if !new_val.is_empty() && new_val != original.get_untracked() {
                                    original.set(new_val.clone());
                                    on_save.run(new_val);
                                }
                            } else if ev.key() == "Escape" {
                                set_editing.set(false);
                            }
                        }
                        node_ref={
                            let node = NodeRef::<leptos::html::Input>::new();
                            leptos::task::spawn_local(async move {
                                if let Some(el) = node.get() {
                                    let _ = el.focus();
                                    let _ = el.select();
                                }
                            });
                            node
                        }
                    />
                }.into_any()
            } else {
                let display_class = class.clone();
                let display_value = original.get();
                view! {
                    <span
                        class=format!("{display_class} cursor-pointer hover:text-primary transition-colors break-words")
                        title="Kliknij aby zmienić nazwę"
                        on:click=move |_| {
                            edit_value.set(original.get_untracked());
                            set_editing.set(true);
                        }
                    >
                        {display_value}
                    </span>
                }.into_any()
            }
        }}
    }
}
