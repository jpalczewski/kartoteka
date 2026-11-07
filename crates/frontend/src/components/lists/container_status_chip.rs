use leptos::prelude::*;
use leptos_fluent::I18n;

use crate::components::ui::{Chip, ChipTone};

/// Status of a project container (`active` / `done` / `paused`) as a chip.
///
/// Folders have no status and never render this. An unknown status (written by
/// another client) is shown verbatim in a neutral chip.
#[component]
pub fn ContainerStatusChip(
    #[prop(into)] status: String,
    /// Optional `data-testid` put on the chip.
    #[prop(optional_no_strip)]
    testid: Option<&'static str>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let (icon, tone, key) = match status.as_str() {
        "active" => (
            "play",
            ChipTone::Primary,
            Some("lists-container-status-active"),
        ),
        "done" => (
            "circle-check",
            ChipTone::Success,
            Some("lists-container-status-done"),
        ),
        "paused" => (
            "hourglass",
            ChipTone::Warning,
            Some("lists-container-status-paused"),
        ),
        _ => ("folder-kanban", ChipTone::Neutral, None),
    };
    let label = Signal::derive(move || match key {
        Some(key) => i18n.tr(key),
        None => status.clone(),
    });

    view! {
        <Chip icon=icon tone=tone testid=testid>
            {move || label.get()}
        </Chip>
    }
}
