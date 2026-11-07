use crate::types::{Item, ItemChange, RecurrenceMode, RecurrenceRule, RecurrenceUnit};
use std::str::FromStr;

fn rule(interval: u16, unit: RecurrenceUnit) -> RecurrenceRule {
    RecurrenceRule { interval, unit }
}

#[test]
fn rule_parses_all_units() {
    assert_eq!(
        RecurrenceRule::from_str("7d"),
        Ok(rule(7, RecurrenceUnit::Day))
    );
    assert_eq!(
        RecurrenceRule::from_str("2w"),
        Ok(rule(2, RecurrenceUnit::Week))
    );
    assert_eq!(
        RecurrenceRule::from_str("1m"),
        Ok(rule(1, RecurrenceUnit::Month))
    );
    assert_eq!(
        RecurrenceRule::from_str("999d"),
        Ok(rule(999, RecurrenceUnit::Day))
    );
}

#[test]
fn rule_display_roundtrips() {
    for s in ["1d", "7d", "2w", "12m", "999w"] {
        assert_eq!(RecurrenceRule::from_str(s).unwrap().to_string(), s);
    }
}

#[test]
fn rule_rejects_invalid() {
    for s in [
        "", "d", "7", "0d", "07d", "1000d", "1y", " 7d", "7d ", "7D", "-1d", "+1d", "1.5d", "7dd",
    ] {
        assert!(
            RecurrenceRule::from_str(s).is_err(),
            "{s:?} should be rejected"
        );
    }
}

#[test]
fn rule_serde_is_string() {
    let r = rule(2, RecurrenceUnit::Week);
    assert_eq!(serde_json::to_string(&r).unwrap(), "\"2w\"");
    assert_eq!(serde_json::from_str::<RecurrenceRule>("\"2w\"").unwrap(), r);
    assert!(serde_json::from_str::<RecurrenceRule>("\"2y\"").is_err());
}

#[test]
fn mode_parse_display_serde() {
    for (s, m) in [
        ("fixed", RecurrenceMode::Fixed),
        ("floating", RecurrenceMode::Floating),
    ] {
        assert_eq!(RecurrenceMode::from_str(s), Ok(m));
        assert_eq!(m.to_string(), s);
        assert_eq!(serde_json::to_string(&m).unwrap(), format!("\"{s}\""));
        assert_eq!(
            serde_json::from_str::<RecurrenceMode>(&format!("\"{s}\"")).unwrap(),
            m
        );
    }
    assert!(RecurrenceMode::from_str("Fixed").is_err());
    assert_eq!(RecurrenceMode::default(), RecurrenceMode::Fixed);
}

fn item(id: &str) -> Item {
    serde_json::from_value(serde_json::json!({
        "id": id, "list_id": "l", "title": "t", "description": null, "completed": false,
        "position": 0, "quantity": null, "actual_quantity": null, "unit": null,
        "start_date": null, "start_time": null, "deadline": "2026-11-07", "deadline_time": null,
        "hard_deadline": null, "estimated_duration": null, "location_id": null,
        "created_at": "c", "updated_at": "u"
    }))
    .unwrap()
}

#[test]
fn item_change_without_spawn_serializes_as_bare_item() {
    let change = ItemChange {
        item: item("a"),
        spawned: None,
    };
    let json = serde_json::to_value(&change).unwrap();
    assert_eq!(json, serde_json::to_value(item("a")).unwrap());
}

#[test]
fn item_change_with_spawn_roundtrips() {
    let change = ItemChange {
        item: item("a"),
        spawned: Some(item("b")),
    };
    let json = serde_json::to_value(&change).unwrap();
    assert_eq!(json["id"], "a");
    assert_eq!(json["spawned"]["id"], "b");
    let back: ItemChange = serde_json::from_value(json).unwrap();
    assert_eq!(back.item.id, "a");
    assert_eq!(back.spawned.unwrap().id, "b");
}
