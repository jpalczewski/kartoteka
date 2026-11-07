//! The "add item" form: a single-line quick add (Enter submits) with an expandable area for
//! description, quantity, dates, recurrence and estimate.
//!
//! The text is cleared only once the server confirmed the new item; on an error the draft
//! stays and the error shows as a toast. Everything the form collects goes through
//! [`build_request`], the one place that turns the typed input into a
//! [`CreateItemRequest`], so a smarter title parser can be plugged in there later.

use kartoteka_shared::{ChangeTopic, CreateItemRequest, default_unit, parse_estimate_minutes};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use super::date_field::DateFieldInput;
use super::fields::{
    EstimateField, RecurrenceFields, estimate_is_valid, recurrence_issue, recurrence_request,
};
use super::item_view::ItemCaps;
use crate::app::{ToastKind, use_toast};
use crate::components::ui::Icon;
use crate::server_fns::items::create_item;
use crate::state::invalidation::mutate_with;
use crate::state::user_today::use_locale;

/// What the user has typed into an [`ItemComposer`] but not submitted yet.
///
/// A page that re-creates its content on refetch owns the draft, so a live update does not
/// wipe the form (#426).
#[derive(Clone, Copy)]
pub struct AddItemDraft {
    pub title: RwSignal<String>,
    pub description: RwSignal<String>,
    pub quantity: RwSignal<String>,
    pub unit: RwSignal<String>,
    pub start_date: RwSignal<String>,
    pub deadline: RwSignal<String>,
    pub hard_deadline: RwSignal<String>,
    pub estimate: RwSignal<String>,
    pub recurrence: RwSignal<String>,
    pub recurrence_mode: RwSignal<String>,
    pub show_more: RwSignal<bool>,
}

const DEFAULT_MODE: &str = "fixed";

impl AddItemDraft {
    pub fn new() -> Self {
        Self {
            title: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            quantity: RwSignal::new(String::new()),
            unit: RwSignal::new(String::new()),
            start_date: RwSignal::new(String::new()),
            deadline: RwSignal::new(String::new()),
            hard_deadline: RwSignal::new(String::new()),
            estimate: RwSignal::new(String::new()),
            recurrence: RwSignal::new(String::new()),
            recurrence_mode: RwSignal::new(DEFAULT_MODE.to_string()),
            show_more: RwSignal::new(false),
        }
    }

    /// Reset every field to its empty state.
    pub fn clear(self) {
        self.title.set(String::new());
        self.description.set(String::new());
        self.quantity.set(String::new());
        self.unit.set(String::new());
        self.start_date.set(String::new());
        self.deadline.set(String::new());
        self.hard_deadline.set(String::new());
        self.estimate.set(String::new());
        self.recurrence.set(String::new());
        self.recurrence_mode.set(DEFAULT_MODE.to_string());
        self.show_more.set(false);
    }

    fn input(self) -> ComposerInput {
        ComposerInput {
            title: self.title.get_untracked(),
            description: self.description.get_untracked(),
            quantity: self.quantity.get_untracked(),
            unit: self.unit.get_untracked(),
            start_date: self.start_date.get_untracked(),
            deadline: self.deadline.get_untracked(),
            hard_deadline: self.hard_deadline.get_untracked(),
            estimate: self.estimate.get_untracked(),
            recurrence: self.recurrence.get_untracked(),
            recurrence_mode: self.recurrence_mode.get_untracked(),
        }
    }
}

impl Default for AddItemDraft {
    fn default() -> Self {
        Self::new()
    }
}

/// The raw text of every composer field.
#[derive(Clone, Debug, Default)]
struct ComposerInput {
    title: String,
    description: String,
    quantity: String,
    unit: String,
    start_date: String,
    deadline: String,
    hard_deadline: String,
    estimate: String,
    recurrence: String,
    recurrence_mode: String,
}

/// Why the typed input cannot be sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputError {
    /// Nothing to add yet; not worth an error message.
    EmptyTitle,
    /// A message key for the toast.
    Invalid(&'static str),
}

fn non_blank(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Turn the typed input into the request. Only the fields the list's features allow are
/// sent: quantity needs the quantity feature, dates and recurrence the deadlines feature,
/// the estimate time tracking.
fn build_request(input: &ComposerInput, caps: ItemCaps) -> Result<CreateItemRequest, InputError> {
    let title = input.title.trim();
    if title.is_empty() {
        return Err(InputError::EmptyTitle);
    }
    let mut request = CreateItemRequest {
        title: title.to_string(),
        description: non_blank(&input.description),
        ..Default::default()
    };

    if caps.quantity {
        request.quantity = input.quantity.trim().parse::<i32>().ok().filter(|q| *q > 0);
        if request.quantity.is_some() {
            request.unit = non_blank(&input.unit);
        }
    }
    if caps.dates {
        request.start_date = non_blank(&input.start_date);
        request.deadline = non_blank(&input.deadline);
        request.hard_deadline = non_blank(&input.hard_deadline);
        let has_date = request.start_date.is_some() || request.deadline.is_some();
        if let Some(issue) = recurrence_issue(&input.recurrence, has_date) {
            return Err(InputError::Invalid(issue.message_key()));
        }
        (request.recurrence_rule, request.recurrence_mode) =
            recurrence_request(&input.recurrence, &input.recurrence_mode);
    }
    if caps.time {
        if !estimate_is_valid(&input.estimate) {
            return Err(InputError::Invalid("item-estimate-invalid"));
        }
        request.estimated_duration = parse_estimate_minutes(&input.estimate);
    }
    Ok(request)
}

/// Add-item form for one list.
///
/// `caps` (from the list's features) decides which fields the "more options" area offers.
/// `compact` is the bare single-line form for sublists and list previews. `draft` is owned
/// by the caller when the form can be re-created while something is typed in it. A created
/// item invalidates [`ChangeTopic::Items`].
#[component]
pub fn ItemComposer(
    #[prop(into)] list_id: Signal<String>,
    #[prop(optional, into)] caps: Option<Signal<ItemCaps>>,
    #[prop(default = false)] compact: bool,
    #[prop(optional)] draft: Option<AddItemDraft>,
    /// `data-testid` of the form.
    #[prop(default = "add-item-form")]
    testid: &'static str,
) -> impl IntoView {
    let toast = use_toast();
    let i18n = expect_context::<I18n>();
    let locale = use_locale();
    let draft = draft.unwrap_or_default();
    let pending = RwSignal::new(false);
    let caps = move || caps.map(|c| c.get()).unwrap_or(ItemCaps::NONE);
    let quantity = Memo::new(move |_| caps().quantity);
    let dates = Memo::new(move |_| caps().dates);
    let time = Memo::new(move |_| caps().time);
    let title_testid = (!compact).then_some("add-item-title");
    let has_recurrence_date = Signal::derive(move || {
        draft.start_date.with(|d| !d.trim().is_empty())
            || draft.deadline.with(|d| !d.trim().is_empty())
    });

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if pending.get_untracked() {
            return;
        }
        let request = match build_request(&draft.input(), caps()) {
            Ok(request) => request,
            Err(InputError::EmptyTitle) => return,
            Err(InputError::Invalid(key)) => {
                toast.push(i18n.tr(key), ToastKind::Error);
                return;
            }
        };
        let list_id = list_id.get_untracked();
        pending.set(true);
        mutate_with(
            async move {
                let created = create_item(list_id, request).await;
                // The form may have been re-created meanwhile (the draft outlives it).
                _ = pending.try_set(false);
                created
            },
            &[ChangeTopic::Items],
            move |_| draft.clear(),
        );
    };

    view! {
        <form on:submit=on_submit class="flex flex-col gap-2" data-testid=testid>
            <div class="flex gap-2">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    data-testid=title_testid
                    placeholder=move_tr!("lists-preview-new-item-placeholder")
                    prop:value=move || draft.title.get()
                    on:input=move |ev| draft.title.set(event_target_value(&ev))
                />
                {(!compact).then(|| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-square"
                        data-testid="composer-more-toggle"
                        aria-expanded=move || draft.show_more.get().to_string()
                        aria-label=move || {
                            if draft.show_more.get() { i18n.tr("composer-fewer") } else { i18n.tr("composer-more") }
                        }
                        title=move || {
                            if draft.show_more.get() { i18n.tr("composer-fewer") } else { i18n.tr("composer-more") }
                        }
                        on:click=move |_| draft.show_more.update(|v| *v = !*v)
                    >
                        <Icon
                            name=Signal::derive(move || {
                                if draft.show_more.get() { "chevron-up".to_string() } else { "chevron-down".to_string() }
                            })
                            size=16
                        />
                    </button>
                })}
                <button type="submit" class="btn btn-primary" prop:disabled=move || pending.get()>
                    {move_tr!("common-add")}
                </button>
            </div>
            <Show when=move || !compact && draft.show_more.get()>
                <div class="flex flex-col gap-2" data-testid="composer-more">
                    <textarea
                        class="textarea textarea-bordered text-sm"
                        data-testid="composer-description"
                        placeholder=move_tr!("composer-description-placeholder")
                        prop:value=move || draft.description.get()
                        on:input=move |ev| draft.description.set(event_target_value(&ev))
                    />
                    <Show when=move || quantity.get()>
                        <div class="flex gap-2">
                            <input
                                type="number"
                                class="input input-bordered input-sm w-24"
                                data-testid="composer-quantity"
                                placeholder=move_tr!("composer-quantity-placeholder")
                                min="1"
                                prop:value=move || draft.quantity.get()
                                on:input=move |ev| draft.quantity.set(event_target_value(&ev))
                            />
                            <input
                                type="text"
                                class="input input-bordered input-sm w-20"
                                data-testid="composer-unit"
                                placeholder=move || default_unit(locale.get())
                                prop:value=move || draft.unit.get()
                                on:input=move |ev| draft.unit.set(event_target_value(&ev))
                            />
                        </div>
                    </Show>
                    <Show when=move || dates.get()>
                        <div class="flex flex-col gap-1">
                            <DateFieldInput label=move_tr!("item-dates-start") value=draft.start_date />
                            <DateFieldInput
                                label=move_tr!("item-dates-deadline")
                                value=draft.deadline
                                data_testid="add-item-deadline"
                            />
                            <DateFieldInput label=move_tr!("item-dates-hard-deadline") value=draft.hard_deadline />
                            <RecurrenceFields
                                rule=draft.recurrence
                                mode=draft.recurrence_mode
                                has_date=has_recurrence_date
                                testid="composer-recurrence-input"
                            />
                        </div>
                    </Show>
                    <Show when=move || time.get()>
                        <EstimateField value=draft.estimate testid="composer-estimate-input" />
                    </Show>
                </div>
            </Show>
        </form>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_caps() -> ItemCaps {
        ItemCaps {
            quantity: true,
            dates: true,
            time: true,
            ..ItemCaps::NONE
        }
    }

    fn input(title: &str) -> ComposerInput {
        ComposerInput {
            title: title.into(),
            recurrence_mode: "fixed".into(),
            ..Default::default()
        }
    }

    #[test]
    fn blank_title_is_not_sent() {
        assert_eq!(
            build_request(&input("  "), all_caps()).unwrap_err(),
            InputError::EmptyTitle
        );
    }

    #[test]
    fn title_only_sends_just_the_title() {
        let request = build_request(&input("  Milk "), ItemCaps::NONE).unwrap();
        assert_eq!(request.title, "Milk");
        assert!(request.description.is_none() && request.unit.is_none());
        assert!(request.recurrence_rule.is_none() && request.estimated_duration.is_none());
    }

    #[test]
    fn expanded_fields_follow_the_list_features() {
        let mut typed = input("Milk");
        typed.description = " 2% ".into();
        typed.quantity = "3".into();
        typed.unit = "l".into();
        typed.deadline = "2030-01-02".into();
        typed.estimate = "1h30m".into();
        typed.recurrence = "2w".into();
        typed.recurrence_mode = "floating".into();

        let request = build_request(&typed, all_caps()).unwrap();
        assert_eq!(request.description.as_deref(), Some("2%"));
        assert_eq!(
            (request.quantity, request.unit.as_deref()),
            (Some(3), Some("l"))
        );
        assert_eq!(request.deadline.as_deref(), Some("2030-01-02"));
        assert_eq!(request.estimated_duration, Some(90));
        assert_eq!(request.recurrence_rule.as_deref(), Some("2w"));
        assert_eq!(request.recurrence_mode.as_deref(), Some("floating"));

        let bare = build_request(&typed, ItemCaps::NONE).unwrap();
        assert!(bare.quantity.is_none() && bare.deadline.is_none());
        assert!(bare.estimated_duration.is_none() && bare.recurrence_rule.is_none());
        assert_eq!(bare.description.as_deref(), Some("2%"));
    }

    #[test]
    fn unit_is_only_sent_with_a_quantity() {
        let mut typed = input("Milk");
        typed.unit = "l".into();
        assert!(build_request(&typed, all_caps()).unwrap().unit.is_none());
    }

    #[test]
    fn bad_estimate_and_recurrence_are_reported() {
        let mut typed = input("Milk");
        typed.estimate = "soon".into();
        assert_eq!(
            build_request(&typed, all_caps()).unwrap_err(),
            InputError::Invalid("item-estimate-invalid")
        );

        let mut typed = input("Milk");
        typed.recurrence = "2w".into();
        assert_eq!(
            build_request(&typed, all_caps()).unwrap_err(),
            InputError::Invalid("item-recurrence-needs-date")
        );
        typed.recurrence = "often".into();
        assert_eq!(
            build_request(&typed, all_caps()).unwrap_err(),
            InputError::Invalid("item-recurrence-invalid")
        );
    }
}
