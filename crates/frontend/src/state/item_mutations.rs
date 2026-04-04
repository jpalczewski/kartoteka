use std::future::Future;

use kartoteka_shared::{DateItem, DayItems, Item, UpdateItemRequest};
use leptos::prelude::*;

use crate::api::ApiError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemDateField {
    Start,
    Deadline,
    HardDeadline,
}

impl ItemDateField {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Self::Start),
            "deadline" => Some(Self::Deadline),
            "hard_deadline" => Some(Self::HardDeadline),
            _ => None,
        }
    }
}

fn next_date_value(date_value: &str) -> Option<String> {
    if date_value.is_empty() {
        None
    } else {
        Some(date_value.to_string())
    }
}

fn next_time_value(date_value: &str, time_value: Option<&str>) -> Option<String> {
    if date_value.is_empty() {
        None
    } else {
        time_value.map(ToOwned::to_owned)
    }
}

pub fn build_date_update_request(
    date_type: &str,
    date_value: &str,
    time_value: Option<String>,
) -> Option<UpdateItemRequest> {
    let field = ItemDateField::parse(date_type)?;
    let date_patch = if date_value.is_empty() {
        Some(None)
    } else {
        Some(Some(date_value.to_string()))
    };
    let time_patch = if date_value.is_empty() {
        Some(None)
    } else {
        time_value.map(Some)
    };

    let mut request = UpdateItemRequest::default();
    match field {
        ItemDateField::Start => {
            request.start_date = date_patch;
            request.start_time = time_patch;
        }
        ItemDateField::Deadline => {
            request.deadline = date_patch;
            request.deadline_time = time_patch;
        }
        ItemDateField::HardDeadline => {
            request.hard_deadline = date_patch;
        }
    }

    Some(request)
}

pub fn apply_item_date_change(
    item: &mut Item,
    field: ItemDateField,
    date_value: &str,
    time_value: Option<&str>,
) {
    let date = next_date_value(date_value);
    let time = next_time_value(date_value, time_value);

    match field {
        ItemDateField::Start => {
            item.start_date = date;
            item.start_time = time;
        }
        ItemDateField::Deadline => {
            item.deadline = date;
            item.deadline_time = time;
        }
        ItemDateField::HardDeadline => {
            item.hard_deadline = date;
        }
    }
}

pub fn apply_date_item_date_change(
    item: &mut DateItem,
    field: ItemDateField,
    date_value: &str,
    time_value: Option<&str>,
) {
    let date = next_date_value(date_value);
    let time = next_time_value(date_value, time_value);

    match field {
        ItemDateField::Start => {
            item.start_date = date;
            item.start_time = time;
        }
        ItemDateField::Deadline => {
            item.deadline = date;
            item.deadline_time = time;
        }
        ItemDateField::HardDeadline => {
            item.hard_deadline = date;
        }
    }
}

pub fn apply_date_change_to_items(
    items: &mut [Item],
    item_id: &str,
    field: ItemDateField,
    date_value: &str,
    time_value: Option<&str>,
) -> bool {
    let mut changed = false;
    for item in items.iter_mut().filter(|item| item.id == item_id) {
        apply_item_date_change(item, field, date_value, time_value);
        changed = true;
    }
    changed
}

pub fn apply_date_change_to_date_items(
    items: &mut [DateItem],
    item_id: &str,
    field: ItemDateField,
    date_value: &str,
    time_value: Option<&str>,
) -> bool {
    let mut changed = false;
    for item in items.iter_mut().filter(|item| item.id == item_id) {
        apply_date_item_date_change(item, field, date_value, time_value);
        changed = true;
    }
    changed
}

#[allow(dead_code)]
pub fn apply_date_change_to_day_items(
    days: &mut [DayItems],
    day: &str,
    item_id: &str,
    field: ItemDateField,
    date_value: &str,
    time_value: Option<&str>,
) -> bool {
    let Some(target_day) = days.iter_mut().find(|current| current.date == day) else {
        return false;
    };

    let Some(item) = target_day.items.iter_mut().find(|item| item.id == item_id) else {
        return false;
    };

    apply_date_item_date_change(item, field, date_value, time_value);
    true
}

pub fn run_optimistic_mutation<T, Mutate, Request, RequestFuture, OnError>(
    signal: RwSignal<T>,
    mutate: Mutate,
    request: Request,
    on_error: OnError,
) where
    T: Clone + Send + Sync + 'static,
    Mutate: FnOnce(&mut T) -> bool + 'static,
    Request: FnOnce() -> RequestFuture + 'static,
    RequestFuture: Future<Output = Result<(), ApiError>> + 'static,
    OnError: FnOnce(ApiError) + 'static,
{
    let previous = signal.get_untracked();
    let mut next = previous.clone();
    if !mutate(&mut next) {
        return;
    }

    signal.set(next);

    leptos::task::spawn_local(async move {
        if let Err(error) = request().await {
            signal.set(previous);
            on_error(error);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::ListType;

    fn sample_item() -> Item {
        Item {
            id: "item-1".into(),
            list_id: "list-1".into(),
            title: "Item".into(),
            description: None,
            completed: false,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: Some("2026-04-01".into()),
            start_time: Some("09:00".into()),
            deadline: Some("2026-04-02".into()),
            deadline_time: Some("12:00".into()),
            hard_deadline: Some("2026-04-03".into()),
            created_at: "2026-04-01".into(),
            updated_at: "2026-04-01".into(),
        }
    }

    fn sample_date_item() -> DateItem {
        DateItem {
            id: "item-1".into(),
            list_id: "list-1".into(),
            title: "Item".into(),
            description: None,
            completed: false,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: Some("2026-04-01".into()),
            start_time: Some("09:00".into()),
            deadline: Some("2026-04-02".into()),
            deadline_time: Some("12:00".into()),
            hard_deadline: Some("2026-04-03".into()),
            created_at: "2026-04-01".into(),
            updated_at: "2026-04-01".into(),
            list_name: "List".into(),
            list_type: ListType::Checklist,
            date_type: Some("deadline".into()),
        }
    }

    #[test]
    fn parse_item_date_field_supports_all_known_variants() {
        assert_eq!(ItemDateField::parse("start"), Some(ItemDateField::Start));
        assert_eq!(
            ItemDateField::parse("deadline"),
            Some(ItemDateField::Deadline)
        );
        assert_eq!(
            ItemDateField::parse("hard_deadline"),
            Some(ItemDateField::HardDeadline)
        );
    }

    #[test]
    fn parse_item_date_field_rejects_unknown_variant() {
        assert_eq!(ItemDateField::parse("start_date"), None);
    }

    #[test]
    fn build_date_update_request_sets_start_date_and_time() {
        let request = build_date_update_request("start", "2026-04-10", Some("08:30".into()))
            .expect("request");

        assert_eq!(request.start_date, Some(Some("2026-04-10".into())));
        assert_eq!(request.start_time, Some(Some("08:30".into())));
        assert_eq!(request.deadline, None);
    }

    #[test]
    fn build_date_update_request_clears_deadline_and_time_when_date_is_empty() {
        let request =
            build_date_update_request("deadline", "", Some("08:30".into())).expect("request");

        assert_eq!(request.deadline, Some(None));
        assert_eq!(request.deadline_time, Some(None));
    }

    #[test]
    fn build_date_update_request_sets_hard_deadline_without_time() {
        let request =
            build_date_update_request("hard_deadline", "2026-04-12", Some("08:30".into()))
                .expect("request");

        assert_eq!(request.hard_deadline, Some(Some("2026-04-12".into())));
        assert_eq!(request.deadline_time, None);
        assert_eq!(request.start_time, None);
    }

    #[test]
    fn build_date_update_request_returns_none_for_invalid_field() {
        assert!(build_date_update_request("unknown", "2026-04-12", None).is_none());
    }

    #[test]
    fn apply_item_date_change_updates_only_selected_field() {
        let mut item = sample_item();

        apply_item_date_change(&mut item, ItemDateField::Start, "2026-04-10", Some("08:30"));

        assert_eq!(item.start_date.as_deref(), Some("2026-04-10"));
        assert_eq!(item.start_time.as_deref(), Some("08:30"));
        assert_eq!(item.deadline.as_deref(), Some("2026-04-02"));
        assert_eq!(item.deadline_time.as_deref(), Some("12:00"));
        assert_eq!(item.hard_deadline.as_deref(), Some("2026-04-03"));
    }

    #[test]
    fn apply_date_item_date_change_clears_paired_time_with_deadline() {
        let mut item = sample_date_item();

        apply_date_item_date_change(&mut item, ItemDateField::Deadline, "", Some("10:00"));

        assert_eq!(item.deadline, None);
        assert_eq!(item.deadline_time, None);
        assert_eq!(item.start_date.as_deref(), Some("2026-04-01"));
    }

    #[test]
    fn apply_date_change_to_day_items_updates_only_matching_day_and_item() {
        let mut days = vec![
            DayItems {
                date: "2026-04-10".into(),
                items: vec![sample_date_item()],
            },
            DayItems {
                date: "2026-04-11".into(),
                items: vec![DateItem {
                    id: "item-2".into(),
                    ..sample_date_item()
                }],
            },
        ];

        let changed = apply_date_change_to_day_items(
            &mut days,
            "2026-04-10",
            "item-1",
            ItemDateField::HardDeadline,
            "2026-04-20",
            None,
        );

        assert!(changed);
        assert_eq!(
            days[0].items[0].hard_deadline.as_deref(),
            Some("2026-04-20")
        );
        assert_eq!(
            days[1].items[0].hard_deadline.as_deref(),
            Some("2026-04-03")
        );
    }
}
