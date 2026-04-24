use leptos::prelude::*;

/// Click-to-edit text field. Calls `on_save` with the new value; parent handles persistence.
/// Re-renders with updated `value` after the parent refetches data.
#[component]
pub fn EditableText(
    value: String,
    on_save: Callback<String>,
    #[prop(default = false)] multiline: bool,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] testid: Option<&'static str>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    let draft = RwSignal::new(value.clone());
    // Store original so cancel closures can restore without capturing String by move.
    let original = StoredValue::new(value);

    let display_class = class.unwrap_or("cursor-pointer hover:underline decoration-dotted");
    let placeholder_text = placeholder.unwrap_or("Kliknij aby edytować...");

    let save = move || {
        let val = draft.get_untracked();
        editing.set(false);
        on_save.run(val);
    };

    let cancel = move || {
        draft.set(original.get_value());
        editing.set(false);
    };

    view! {
        <Show
            when=move || editing.get()
            fallback=move || {
                let is_empty = draft.get().trim().is_empty();
                view! {
                    <span
                        class=format!("{display_class}{}", if is_empty { " italic opacity-50" } else { "" })
                        on:click=move |_| editing.set(true)
                        title="Kliknij aby edytować"
                        attr:data-testid=testid
                    >
                        {move || {
                            let v = draft.get();
                            if v.trim().is_empty() { placeholder_text.to_string() } else { v }
                        }}
                    </span>
                }
            }
        >
            {if multiline {
                let textarea_testid = testid.map(|t| format!("{t}-input"));
                view! {
                    <div class="flex flex-col gap-1">
                        <textarea
                            class="textarea textarea-bordered w-full"
                            prop:value=move || draft.get()
                            on:input=move |ev| draft.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Escape" { cancel(); }
                            }
                            rows=3
                            autofocus=true
                            attr:data-testid=textarea_testid
                        />
                        <div class="flex gap-2">
                            <button type="button" class="btn btn-xs btn-primary" on:click=move |_| save()>"Zapisz"</button>
                            <button type="button" class="btn btn-xs btn-ghost" on:click=move |_| cancel()>"Anuluj"</button>
                        </div>
                    </div>
                }.into_any()
            } else {
                let input_testid = testid.map(|t| format!("{t}-input"));
                view! {
                    <input
                        type="text"
                        class="input input-bordered w-full"
                        prop:value=move || draft.get()
                        on:input=move |ev| draft.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            match ev.key().as_str() {
                                "Enter" => save(),
                                "Escape" => cancel(),
                                _ => {}
                            }
                        }
                        on:blur=move |_| {
                            if editing.get_untracked() { save(); }
                        }
                        autofocus=true
                        attr:data-testid=input_testid
                    />
                }.into_any()
            }}
        </Show>
    }
}
