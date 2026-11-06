use crate::constants::REMINDER_DEFAULTS_KEY;
use crate::models::ReminderDefaults;

#[test]
fn reminder_defaults_key_is_stable() {
    // Stored JSON in `user_settings` uses this key; changing it would orphan saved settings.
    assert_eq!(REMINDER_DEFAULTS_KEY, "reminder_defaults");
}

#[test]
fn reminder_defaults_default_values() {
    let defaults = ReminderDefaults::default();
    assert!(defaults.start_date.is_empty());
    assert_eq!(defaults.deadline, vec![60]);
    assert_eq!(defaults.hard_deadline, vec![1440, 60]);
    assert!(defaults.validate().is_ok());
}

#[test]
fn reminder_defaults_validate_accepts_limits() {
    let defaults = ReminderDefaults {
        start_date: vec![0, 1, 2, 3, 40_320],
        deadline: vec![],
        hard_deadline: vec![],
    };
    assert!(defaults.validate().is_ok());
}

#[test]
fn reminder_defaults_validate_rejects_too_many_entries() {
    let defaults = ReminderDefaults {
        start_date: vec![1, 2, 3, 4, 5, 6],
        ..ReminderDefaults::default()
    };
    assert_eq!(
        defaults.validate(),
        Err("at most 5 reminders per date field")
    );
}

#[test]
fn reminder_defaults_validate_rejects_too_large_offset() {
    let too_large = ReminderDefaults {
        hard_deadline: vec![40_321],
        ..ReminderDefaults::default()
    };
    assert_eq!(
        too_large.validate(),
        Err("reminder offset must be at most 40320 minutes (28 days)")
    );

    let overflow = ReminderDefaults {
        deadline: vec![u32::MAX],
        ..ReminderDefaults::default()
    };
    assert!(overflow.validate().is_err());
}

#[test]
fn reminder_defaults_serde_roundtrip() {
    let defaults = ReminderDefaults {
        start_date: vec![15],
        deadline: vec![60, 120],
        hard_deadline: vec![1440],
    };
    let json = serde_json::to_string(&defaults).unwrap();
    assert_eq!(
        json,
        r#"{"start_date":[15],"deadline":[60,120],"hard_deadline":[1440]}"#
    );
    assert_eq!(
        serde_json::from_str::<ReminderDefaults>(&json).unwrap(),
        defaults
    );
}

#[test]
fn reminder_defaults_serde_missing_fields() {
    let empty = serde_json::from_str::<ReminderDefaults>("{}").unwrap();
    assert_eq!(
        empty,
        ReminderDefaults {
            start_date: vec![],
            deadline: vec![],
            hard_deadline: vec![],
        }
    );

    let partial = serde_json::from_str::<ReminderDefaults>(r#"{"deadline": [30]}"#).unwrap();
    assert!(partial.start_date.is_empty());
    assert_eq!(partial.deadline, vec![30]);
    assert!(partial.hard_deadline.is_empty());
}
