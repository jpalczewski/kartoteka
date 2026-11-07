use leptos::prelude::*;
use leptos_fluent::move_tr;

/// DaisyUI size presets for [`InlineEdit`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InlineEditSize {
    /// `input-sm` input with `btn-sm` buttons.
    #[default]
    Sm,
    /// `input-xs` input with `btn-xs` buttons.
    Xs,
}

impl InlineEditSize {
    const fn input_class(self) -> &'static str {
        match self {
            Self::Sm => "input input-bordered input-sm flex-1",
            Self::Xs => "input input-bordered input-xs flex-1",
        }
    }

    const fn save_class(self) -> &'static str {
        match self {
            Self::Sm => "btn btn-sm btn-primary",
            Self::Xs => "btn btn-xs btn-primary",
        }
    }

    const fn cancel_class(self) -> &'static str {
        match self {
            Self::Sm => "btn btn-sm btn-ghost",
            Self::Xs => "btn btn-xs btn-ghost",
        }
    }

    const fn wrapper_class(self) -> &'static str {
        match self {
            Self::Sm => "flex gap-2 items-center",
            Self::Xs => "flex gap-1 items-center",
        }
    }
}

/// Inline text editor: a text input plus Save/Cancel buttons.
///
/// Enter saves, Escape cancels. The component owns its draft, initialized from
/// `value` on mount — parents close the edit mode in `on_save`/`on_cancel`, which
/// unmounts the component, so reopening always starts from `value` again.
///
/// Props (documented here because rustc forbids doc comments on parameters):
/// - `value`: initial draft value, read only on mount.
/// - `on_save`: called with the edited value on Enter / Save.
/// - `on_cancel`: called on Escape / Cancel.
/// - `size`: DaisyUI size preset, `Sm` by default.
/// - `placeholder`: optional input placeholder.
/// - `testid`: applied to the `<input>` as `data-testid`.
/// - `allow_empty`: when false (default), saving a trimmed-empty value is a no-op.
#[component]
pub fn InlineEdit(
    value: String,
    on_save: Callback<String>,
    on_cancel: Callback<()>,
    #[prop(optional)] size: InlineEditSize,
    #[prop(optional_no_strip)] placeholder: Option<String>,
    #[prop(optional_no_strip)] testid: Option<String>,
    #[prop(default = false)] allow_empty: bool,
) -> impl IntoView {
    let draft = RwSignal::new(value);
    let input_ref = NodeRef::<leptos::html::Input>::new();

    let save = move || {
        let val = draft.get_untracked();
        if !allow_empty && val.trim().is_empty() {
            return;
        }
        on_save.run(val);
    };

    // `autofocus` covers the SSR/hydrated render; the effect covers inputs inserted
    // later by client-side toggling of the edit mode.
    Effect::new(move |_| {
        if let Some(input) = input_ref.get() {
            let _ = input.focus();
        }
    });

    view! {
        <div class=size.wrapper_class()>
            <input
                type="text"
                node_ref=input_ref
                class=size.input_class()
                placeholder=placeholder
                prop:value=move || draft.get()
                on:input=move |ev| draft.set(event_target_value(&ev))
                on:keydown=move |ev| match ev.key().as_str() {
                    "Enter" => save(),
                    "Escape" => on_cancel.run(()),
                    _ => {}
                }
                autofocus=true
                data-testid=testid
            />
            <button type="button" class=size.save_class() on:click=move |_| save()>
                {move_tr!("common-save")}
            </button>
            <button type="button" class=size.cancel_class() on:click=move |_| on_cancel.run(())>
                {move_tr!("common-cancel")}
            </button>
        </div>
    }
}
