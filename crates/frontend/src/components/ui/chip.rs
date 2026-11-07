use leptos::prelude::*;

use super::icon::{Icon, IconName};

/// Colour tone of a [`Chip`], mapped onto DaisyUI badge modifiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChipTone {
    /// Muted grey pill, the default for metadata.
    #[default]
    Neutral,
    /// Highlighted pill.
    Primary,
    /// Upcoming / attention.
    Warning,
    /// Overdue / destructive.
    Error,
    /// Completed / positive.
    Success,
}

impl ChipTone {
    const fn class(self) -> &'static str {
        match self {
            Self::Neutral => "badge-ghost",
            Self::Primary => "badge-primary",
            Self::Warning => "badge-warning",
            Self::Error => "badge-error",
            Self::Success => "badge-success",
        }
    }
}

/// Small pill for a piece of metadata, with an optional leading icon.
#[component]
pub fn Chip(
    /// Optional lucide icon shown before the content.
    #[prop(optional, into)]
    icon: Option<IconName>,
    #[prop(optional)] tone: ChipTone,
    /// Tooltip for the pill (rendered as the `title` attribute).
    #[prop(optional, into)]
    title: Option<String>,
    /// Optional `data-testid` put on the `<span>`.
    #[prop(optional_no_strip)]
    testid: Option<&'static str>,
    /// Extra classes for the pill.
    #[prop(default = String::new(), into)]
    class: String,
    children: Children,
) -> impl IntoView {
    view! {
        <span
            class=format!("badge badge-sm gap-1 {} {class}", tone.class())
            title=title
            data-testid=testid
        >
            {icon.map(|icon| view! { <Icon name=icon size=12 /> })}
            {children()}
        </span>
    }
}
