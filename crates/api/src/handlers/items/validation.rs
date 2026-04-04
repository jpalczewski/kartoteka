use kartoteka_shared::*;

use super::MAX_ITEM_TITLE_LENGTH;

#[derive(Clone, Debug)]
pub(super) struct ItemTemporalState {
    pub(super) start_date: Option<String>,
    pub(super) start_time: Option<String>,
    pub(super) deadline: Option<String>,
    pub(super) deadline_time: Option<String>,
    pub(super) hard_deadline: Option<String>,
}

impl ItemTemporalState {
    pub(super) fn from_create(body: &CreateItemRequest) -> Self {
        Self {
            start_date: body.start_date.clone(),
            start_time: body.start_time.clone(),
            deadline: body.deadline.clone(),
            deadline_time: body.deadline_time.clone(),
            hard_deadline: body.hard_deadline.clone(),
        }
    }

    pub(super) fn from_item(item: &Item) -> Self {
        Self {
            start_date: item.start_date.clone(),
            start_time: item.start_time.clone(),
            deadline: item.deadline.clone(),
            deadline_time: item.deadline_time.clone(),
            hard_deadline: item.hard_deadline.clone(),
        }
    }

    pub(super) fn apply_update(&mut self, body: &UpdateItemRequest) {
        apply_patch_field(&mut self.start_date, &body.start_date);
        apply_patch_field(&mut self.start_time, &body.start_time);
        apply_patch_field(&mut self.deadline, &body.deadline);
        apply_patch_field(&mut self.deadline_time, &body.deadline_time);
        apply_patch_field(&mut self.hard_deadline, &body.hard_deadline);
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ItemQuantityState {
    pub(super) quantity: Option<i32>,
    pub(super) actual_quantity: Option<i32>,
}

impl ItemQuantityState {
    pub(super) fn from_create(body: &CreateItemRequest) -> Self {
        Self {
            quantity: body.quantity,
            actual_quantity: body.quantity.map(|_| 0),
        }
    }

    pub(super) fn from_item(item: &Item) -> Self {
        Self {
            quantity: item.quantity,
            actual_quantity: item.actual_quantity,
        }
    }

    pub(super) fn apply_update(&mut self, body: &UpdateItemRequest) {
        if let Some(quantity) = body.quantity {
            self.quantity = Some(quantity);
        }
        if let Some(actual_quantity) = body.actual_quantity {
            self.actual_quantity = Some(actual_quantity);
        }
    }
}

pub(super) fn apply_patch_field(target: &mut Option<String>, patch: &Option<Option<String>>) {
    match patch {
        Some(Some(value)) => *target = Some(value.clone()),
        Some(None) => *target = None,
        None => {}
    }
}

pub(super) fn validation_field(field: &str, code: &str) -> ValidationFieldError {
    ValidationFieldError {
        field: field.to_string(),
        code: code.to_string(),
    }
}

pub(super) fn normalize_title(
    title: &str,
    field: &str,
    errors: &mut Vec<ValidationFieldError>,
) -> Option<String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        errors.push(validation_field(field, "required"));
        None
    } else if trimmed.chars().count() > MAX_ITEM_TITLE_LENGTH {
        errors.push(validation_field(field, "too_long"));
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn validate_item_quantity_state(
    state: &ItemQuantityState,
    errors: &mut Vec<ValidationFieldError>,
) {
    if state.quantity.is_some_and(|quantity| quantity <= 0) {
        errors.push(validation_field("quantity", "must_be_positive"));
    }
    if state
        .actual_quantity
        .is_some_and(|actual_quantity| actual_quantity < 0)
    {
        errors.push(validation_field("actual_quantity", "must_be_non_negative"));
    }
}

pub(super) fn derive_completed_from_quantity_state(state: &ItemQuantityState) -> bool {
    match (state.quantity, state.actual_quantity) {
        (Some(quantity), Some(actual_quantity)) => actual_quantity >= quantity,
        _ => false,
    }
}

pub(super) fn validate_date_field(
    field: &str,
    value: Option<&str>,
    errors: &mut Vec<ValidationFieldError>,
) -> Option<chrono::NaiveDate> {
    let value = value?;
    match validate_business_date(value) {
        Ok(date) => Some(date),
        Err(DateValidationError::Invalid) => {
            errors.push(validation_field(field, "invalid_date"));
            None
        }
        Err(DateValidationError::OutOfRange) => {
            errors.push(validation_field(field, "date_out_of_range"));
            None
        }
    }
}

pub(super) fn validate_time_field(
    field: &str,
    value: Option<&str>,
    errors: &mut Vec<ValidationFieldError>,
) -> bool {
    let Some(value) = value else {
        return true;
    };
    if validate_hhmm_time(value).is_ok() {
        true
    } else {
        errors.push(validation_field(field, "invalid_time"));
        false
    }
}

pub(super) fn validate_item_temporal_state(state: &ItemTemporalState) -> Vec<ValidationFieldError> {
    let mut errors = Vec::new();
    let start_date = validate_date_field("start_date", state.start_date.as_deref(), &mut errors);
    let deadline = validate_date_field("deadline", state.deadline.as_deref(), &mut errors);
    let hard_deadline =
        validate_date_field("hard_deadline", state.hard_deadline.as_deref(), &mut errors);

    let start_has_valid_time =
        validate_time_field("start_time", state.start_time.as_deref(), &mut errors);
    let deadline_has_valid_time =
        validate_time_field("deadline_time", state.deadline_time.as_deref(), &mut errors);

    if state.start_time.is_some() && state.start_date.is_none() && start_has_valid_time {
        errors.push(validation_field("start_time", "time_requires_date"));
    }
    if state.deadline_time.is_some() && state.deadline.is_none() && deadline_has_valid_time {
        errors.push(validation_field("deadline_time", "time_requires_date"));
    }

    if let (Some(start_date), Some(deadline)) = (start_date, deadline)
        && start_date > deadline
    {
        errors.push(validation_field("start_date", "start_after_deadline"));
    }
    if let (Some(deadline), Some(hard_deadline)) = (deadline, hard_deadline)
        && deadline > hard_deadline
    {
        errors.push(validation_field(
            "hard_deadline",
            "hard_deadline_before_deadline",
        ));
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_title_rejects_titles_longer_than_255_chars() {
        let title = "a".repeat(MAX_ITEM_TITLE_LENGTH + 1);
        let mut errors = Vec::new();

        let normalized = normalize_title(&title, "title", &mut errors);

        assert!(normalized.is_none());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field, "title");
        assert_eq!(errors[0].code, "too_long");
    }

    #[test]
    fn quantity_validation_rejects_non_positive_quantity_and_negative_actual() {
        let state = ItemQuantityState {
            quantity: Some(0),
            actual_quantity: Some(-1),
        };
        let mut errors = Vec::new();

        validate_item_quantity_state(&state, &mut errors);

        assert_eq!(errors.len(), 2);
        assert_eq!(errors[0].field, "quantity");
        assert_eq!(errors[0].code, "must_be_positive");
        assert_eq!(errors[1].field, "actual_quantity");
        assert_eq!(errors[1].code, "must_be_non_negative");
    }

    #[test]
    fn derived_completion_uses_quantity_and_actual_quantity() {
        assert!(!derive_completed_from_quantity_state(&ItemQuantityState {
            quantity: Some(3),
            actual_quantity: Some(2),
        }));
        assert!(derive_completed_from_quantity_state(&ItemQuantityState {
            quantity: Some(3),
            actual_quantity: Some(3),
        }));
        assert!(!derive_completed_from_quantity_state(&ItemQuantityState {
            quantity: None,
            actual_quantity: None,
        }));
    }
}
