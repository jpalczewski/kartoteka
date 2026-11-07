use kartoteka_shared::human_date::DateUrgency;
use kartoteka_shared::item_display::{DateChip, DateChipKind, item_date_chips};
use kartoteka_shared::types::Item;
use leptos::prelude::*;

use crate::components::ui::{Chip, ChipTone};
use crate::state::user_today::{use_locale, use_today};

/// Tone of a date chip: overdue is an error, due today a warning, due soon primary, anything
/// else (later, or no urgency such as a completed item) neutral.
const fn urgency_tone(urgency: DateUrgency) -> ChipTone {
    match urgency {
        DateUrgency::Overdue => ChipTone::Error,
        DateUrgency::Today => ChipTone::Warning,
        DateUrgency::Soon => ChipTone::Primary,
        DateUrgency::Later | DateUrgency::None => ChipTone::Neutral,
    }
}

/// Chip text: the date label, followed by the time when there is one.
fn chip_label(label: &str, time: Option<&str>) -> String {
    match time {
        Some(time) => format!("{label} {time}"),
        None => label.to_string(),
    }
}

/// Lucide icon for each kind of date.
const fn chip_icon(kind: &DateChipKind) -> &'static str {
    match kind {
        DateChipKind::HardDeadline => "alarm-clock",
        DateChipKind::Deadline => "clock",
        DateChipKind::Start => "calendar",
    }
}

#[derive(Clone, Debug, PartialEq)]
struct DateChipView {
    /// `None` in compact mode.
    icon: Option<&'static str>,
    tone: ChipTone,
    dim: bool,
    text: String,
}

fn chip_view(chip: &DateChip, completed: bool, compact: bool) -> DateChipView {
    DateChipView {
        icon: (!compact).then(|| chip_icon(&chip.kind)),
        tone: urgency_tone(chip.urgency),
        dim: completed,
        text: chip_label(&chip.label, chip.time.as_deref()),
    }
}

/// Date chips for an item: hard deadline, deadline, start. Nothing until the user's today is
/// known. `compact` drops the icons for dense rows.
#[component]
pub fn DateChips(
    #[prop(into)] item: Signal<Option<Item>>,
    #[prop(optional)] compact: bool,
) -> impl IntoView {
    let today = use_today();
    let locale = use_locale();
    let chips = Memo::new(move |_| {
        let today = today.get();
        let locale = locale.get();
        item.with(|i| match (i, today) {
            (Some(i), Some(today)) => item_date_chips(i, today, locale)
                .iter()
                .map(|chip| chip_view(chip, i.completed, compact))
                .collect(),
            _ => Vec::new(),
        })
    });

    move || {
        chips
            .get()
            .into_iter()
            .map(|chip| {
                let class = if chip.dim { "opacity-40" } else { "" };
                match chip.icon {
                    Some(icon) => view! {
                        <Chip icon=icon tone=chip.tone class=class>{chip.text}</Chip>
                    }
                    .into_any(),
                    None => view! {
                        <Chip tone=chip.tone class=class>{chip.text}</Chip>
                    }
                    .into_any(),
                }
            })
            .collect_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urgency_maps_to_tone() {
        assert_eq!(urgency_tone(DateUrgency::Overdue), ChipTone::Error);
        assert_eq!(urgency_tone(DateUrgency::Today), ChipTone::Warning);
        assert_eq!(urgency_tone(DateUrgency::Soon), ChipTone::Primary);
        assert_eq!(urgency_tone(DateUrgency::Later), ChipTone::Neutral);
        assert_eq!(urgency_tone(DateUrgency::None), ChipTone::Neutral);
    }

    #[test]
    fn label_appends_time_only_when_present() {
        assert_eq!(chip_label("Today", None), "Today");
        assert_eq!(chip_label("Today", Some("14:30")), "Today 14:30");
    }

    #[test]
    fn each_kind_has_its_icon() {
        assert_eq!(chip_icon(&DateChipKind::HardDeadline), "alarm-clock");
        assert_eq!(chip_icon(&DateChipKind::Deadline), "clock");
        assert_eq!(chip_icon(&DateChipKind::Start), "calendar");
    }
}
