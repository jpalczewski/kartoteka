use leptos::prelude::*;
use leptos_fluent::move_tr;

use super::inline_edit::InlineEdit;

/// Click-to-edit text field. Calls `on_save` with the new value; parent handles persistence.
/// Follows `value` when it changes (after a refetch, or an edit from another tab) unless the
/// field is being edited, so a page can keep this component alive across refetches.
///
/// Single-line edit mode delegates to [`InlineEdit`] (Enter saves, Escape cancels).
/// There is deliberately **no** save-on-blur: clicking Cancel must never save first.
/// Multiline mode keeps its own textarea with the same Save/Cancel semantics.
#[component]
pub fn EditableText(
    #[prop(into)] value: Signal<String>,
    on_save: Callback<String>,
    #[prop(default = false)] multiline: bool,
    #[prop(optional)] placeholder: Option<&'static str>,
    #[prop(optional)] class: Option<&'static str>,
    #[prop(optional)] testid: Option<&'static str>,
) -> impl IntoView {
    let editing = RwSignal::new(false);
    // Value shown while not editing; updated on save so the new text appears before refetch.
    let current = RwSignal::new(value.get_untracked());
    // Last committed value, restored when a multiline edit is cancelled.
    let committed = StoredValue::new(current.get_untracked());
    Effect::new(move |_| {
        let latest = value.get();
        committed.set_value(latest.clone());
        if !editing.get_untracked() {
            current.set(latest);
        }
    });

    let display_class = class.unwrap_or("cursor-pointer hover:underline decoration-dotted");
    let fallback_placeholder = move_tr!("common-click-to-edit");

    let display_text = move || {
        let val = current.get();
        if val.trim().is_empty() {
            placeholder.map_or_else(|| fallback_placeholder.get(), str::to_string)
        } else {
            val
        }
    };

    let save_value = move |val: String| {
        committed.set_value(val.clone());
        current.set(val.clone());
        editing.set(false);
        on_save.run(val);
    };

    let cancel_edit = move || {
        current.set(committed.get_value());
        editing.set(false);
    };

    let input_testid = testid.map(|t| format!("{t}-input"));
    let textarea_testid = input_testid.clone();

    view! {
        <Show
            when=move || editing.get()
            fallback=move || {
                let is_empty = current.get().trim().is_empty();
                view! {
                    <span
                        class=format!("{display_class}{}", if is_empty { " italic opacity-50" } else { "" })
                        on:click=move |_| editing.set(true)
                        title=move_tr!("common-click-to-edit-title")
                        data-testid=testid
                    >
                        {display_text}
                    </span>
                }
            }
        >
            {if multiline {
                view! {
                    <div class="flex flex-col gap-1">
                        <textarea
                            class="textarea textarea-bordered w-full"
                            prop:value=move || current.get()
                            on:input=move |ev| current.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Escape" { cancel_edit(); }
                            }
                            rows=3
                            autofocus=true
                            data-testid=textarea_testid.clone()
                        />
                        <div class="flex gap-2">
                            <button
                                type="button"
                                class="btn btn-xs btn-primary"
                                on:click=move |_| save_value(current.get_untracked())
                            >
                                {move_tr!("common-save")}
                            </button>
                            <button type="button" class="btn btn-xs btn-ghost" on:click=move |_| cancel_edit()>
                                {move_tr!("common-cancel")}
                            </button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {
                    <InlineEdit
                        value=current.get_untracked()
                        on_save=Callback::new(move |val: String| save_value(val))
                        on_cancel=Callback::new(move |_: ()| cancel_edit())
                        allow_empty=true
                        testid=input_testid.clone()
                    />
                }.into_any()
            }}
        </Show>
    }
}
