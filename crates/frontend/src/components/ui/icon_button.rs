use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::icon::{Icon, IconName};

/// Visual variant of an [`IconButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonVariant {
    /// Neutral ghost button, the default for secondary row actions.
    #[default]
    Ghost,
    /// Ghost button with destructive (error) colouring.
    Danger,
}

/// Size preset of an [`IconButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonSize {
    /// `btn-xs` with a 14px icon — dense rows and card corners.
    #[default]
    Xs,
    /// `btn-sm` with a 16px icon — page headers.
    Sm,
}

impl IconButtonVariant {
    const fn class(self) -> &'static str {
        match self {
            Self::Ghost => "btn-ghost",
            Self::Danger => "btn-ghost text-error",
        }
    }
}

impl IconButtonSize {
    const fn button_class(self) -> &'static str {
        match self {
            Self::Xs => "btn-xs",
            Self::Sm => "btn-sm",
        }
    }

    const fn icon_size(self) -> usize {
        match self {
            Self::Xs => 14,
            Self::Sm => 16,
        }
    }
}

/// Square icon-only button with a mandatory accessible label.
///
/// `label` is rendered as both `aria-label` and `title`, so every glyph that used
/// to be its own visible text now has a real name. `testid` lands on the
/// `<button>` as `data-testid`, which keeps existing e2e selectors working.
#[component]
pub fn IconButton(
    #[prop(into)] icon: IconName,
    /// Accessible name (also the tooltip).
    #[prop(into)]
    label: Signal<String>,
    /// Click handler; receives the click event so callers can stop propagation.
    #[prop(optional)]
    on_click: Option<Callback<MouseEvent>>,
    #[prop(optional)] variant: IconButtonVariant,
    #[prop(optional)] size: IconButtonSize,
    /// Optional `data-testid` put on the `<button>`.
    #[prop(optional_no_strip)]
    testid: Option<&'static str>,
    /// Extra classes for the `<button>`.
    #[prop(default = String::new(), into)]
    class: String,
) -> impl IntoView {
    view! {
        <button
            type="button"
            class=format!(
                "btn btn-square {} {} {class}",
                size.button_class(),
                variant.class(),
            )
            aria-label=label
            title=label
            data-testid=testid
            on:click=move |ev: MouseEvent| {
                if let Some(cb) = on_click {
                    cb.run(ev);
                }
            }
        >
            <Icon name=icon size=size.icon_size() />
        </button>
    }
}
