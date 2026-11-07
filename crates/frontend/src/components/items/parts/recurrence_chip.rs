use kartoteka_shared::types::{Item, RecurrenceMode};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::components::ui::Chip;

#[derive(Clone, Debug, PartialEq)]
struct RecurrenceView {
    rule: String,
    mode_key: &'static str,
    completed: bool,
}

/// Read-only recurrence badge: the rule (`2w`) plus the mode as a tooltip. Nothing for a
/// one-off item.
#[component]
pub fn RecurrenceChip(#[prop(into)] item: Signal<Option<Item>>) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let recurrence = Memo::new(move |_| {
        item.with(|i| {
            i.as_ref().and_then(|i| {
                i.recurrence_rule.map(|rule| RecurrenceView {
                    rule: rule.to_string(),
                    mode_key: match i.recurrence_mode.unwrap_or_default() {
                        RecurrenceMode::Fixed => "item-recurrence-mode-fixed",
                        RecurrenceMode::Floating => "item-recurrence-mode-floating",
                    },
                    completed: i.completed,
                })
            })
        })
    });

    move || {
        recurrence.get().map(|rec| {
            // The macro's arg closure needs its own copy: it is re-run on locale changes.
            let every_rule = rec.rule.clone();
            let every = move_tr!("item-recurrence-every", { "rule" => every_rule.clone() });
            let title = format!("{} · {}", every.get(), i18n.tr(rec.mode_key));
            let class = if rec.completed { "opacity-40" } else { "" };
            view! {
                <div class="flex gap-1 shrink-0">
                    <Chip
                        icon="repeat"
                        title=title
                        testid=Some("item-recurrence-chip")
                        class=class
                    >
                        {rec.rule}
                    </Chip>
                </div>
            }
        })
    }
}
