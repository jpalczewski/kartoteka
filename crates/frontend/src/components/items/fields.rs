//! Inputs for the item fields that the composer and the row editor share: recurrence rule and
//! estimate. They only edit the signals they are given; saving is up to the caller.

use std::str::FromStr;

use kartoteka_shared::parse_estimate_minutes;
use kartoteka_shared::types::{RecurrenceMode, RecurrenceRule};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

/// Why a recurrence input cannot be saved yet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecurrenceIssue {
    /// Not `<number><d|w|m>`.
    Invalid,
    /// A rule needs a start date or deadline (the domain rejects it otherwise).
    NeedsDate,
}

impl RecurrenceIssue {
    pub fn message_key(self) -> &'static str {
        match self {
            Self::Invalid => "item-recurrence-invalid",
            Self::NeedsDate => "item-recurrence-needs-date",
        }
    }
}

/// Check a typed rule. Blank means "no recurrence" and is always fine.
pub fn recurrence_issue(raw: &str, has_date: bool) -> Option<RecurrenceIssue> {
    let raw = raw.trim();
    if raw.is_empty() {
        None
    } else if RecurrenceRule::from_str(raw).is_err() {
        Some(RecurrenceIssue::Invalid)
    } else if !has_date {
        Some(RecurrenceIssue::NeedsDate)
    } else {
        None
    }
}

/// The `(rule, mode)` pair to send for a typed rule and the selected mode; both `None` for a
/// blank rule, so a cleared field never leaves a dangling mode.
pub fn recurrence_request(raw: &str, mode: &str) -> (Option<String>, Option<String>) {
    let raw = raw.trim();
    if raw.is_empty() {
        (None, None)
    } else {
        (Some(raw.to_string()), Some(mode.to_string()))
    }
}

/// Wire value of the mode select for an item's current mode.
pub fn mode_value(mode: Option<RecurrenceMode>) -> String {
    mode.unwrap_or_default().to_string()
}

/// Whether a typed estimate is acceptable: blank (none) or a valid duration.
pub fn estimate_is_valid(raw: &str) -> bool {
    raw.trim().is_empty() || parse_estimate_minutes(raw).is_some()
}

/// Recurrence rule input with the fixed/floating mode select and a hint line.
/// `has_date` tells whether the item has (or will have) a start date or deadline.
#[component]
pub fn RecurrenceFields(
    rule: RwSignal<String>,
    mode: RwSignal<String>,
    #[prop(into)] has_date: Signal<bool>,
    /// `data-testid` of the rule input; the mode select gets `<testid>-mode`.
    testid: &'static str,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let issue = Memo::new(move |_| rule.with(|r| recurrence_issue(r, has_date.get())));
    let mode_testid = format!("{testid}-mode");

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2 flex-wrap">
                <span class="text-base-content/50 text-xs w-28">{move_tr!("item-recurrence-label")}</span>
                <input
                    type="text"
                    class="input input-bordered input-xs w-24"
                    class:input-error=move || issue.get().is_some()
                    placeholder="2w"
                    data-testid=testid
                    prop:value=move || rule.get()
                    on:input=move |ev| rule.set(event_target_value(&ev))
                />
                <select
                    class="select select-bordered select-xs w-auto"
                    title=move_tr!("item-recurrence-mode-title")
                    data-testid=mode_testid
                    on:change=move |ev| mode.set(event_target_value(&ev))
                >
                    <option value="fixed" selected=move || mode.get() == "fixed">
                        {move_tr!("item-recurrence-mode-fixed")}
                    </option>
                    <option value="floating" selected=move || mode.get() == "floating">
                        {move_tr!("item-recurrence-mode-floating")}
                    </option>
                </select>
            </div>
            {move || issue.get().map(|issue| view! {
                <p class="text-xs text-error" data-testid="item-recurrence-issue">
                    {i18n.tr(issue.message_key())}
                </p>
            })}
        </div>
    }
}

/// Estimate input (`45`, `2h`, `1h30m`) with an error state.
#[component]
pub fn EstimateField(
    value: RwSignal<String>,
    /// `data-testid` of the input.
    testid: &'static str,
) -> impl IntoView {
    let invalid = Memo::new(move |_| value.with(|v| !estimate_is_valid(v)));

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex items-center gap-2">
                <span class="text-base-content/50 text-xs w-28">{move_tr!("item-estimate-label")}</span>
                <input
                    type="text"
                    class="input input-bordered input-xs w-24"
                    class:input-error=move || invalid.get()
                    placeholder=move_tr!("item-estimate-placeholder")
                    data-testid=testid
                    prop:value=move || value.get()
                    on:input=move |ev| value.set(event_target_value(&ev))
                />
            </div>
            <Show when=move || invalid.get()>
                <p class="text-xs text-error">{move_tr!("item-estimate-invalid")}</p>
            </Show>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_rule_is_never_an_issue() {
        assert_eq!(recurrence_issue("  ", false), None);
    }

    #[test]
    fn unparsable_rule_is_invalid_even_with_a_date() {
        assert_eq!(
            recurrence_issue("every day", true),
            Some(RecurrenceIssue::Invalid)
        );
        assert_eq!(recurrence_issue("0d", true), Some(RecurrenceIssue::Invalid));
    }

    #[test]
    fn valid_rule_needs_a_date() {
        assert_eq!(
            recurrence_issue("2w", false),
            Some(RecurrenceIssue::NeedsDate)
        );
        assert_eq!(recurrence_issue(" 2w ", true), None);
    }

    #[test]
    fn blank_rule_drops_the_mode() {
        assert_eq!(recurrence_request(" ", "floating"), (None, None));
        assert_eq!(
            recurrence_request(" 7d ", "floating"),
            (Some("7d".into()), Some("floating".into()))
        );
    }

    #[test]
    fn estimate_validity() {
        assert!(estimate_is_valid(""));
        assert!(estimate_is_valid("1h30m"));
        assert!(!estimate_is_valid("soon"));
    }
}
