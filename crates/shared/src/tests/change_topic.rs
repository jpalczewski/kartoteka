use crate::models::ChangeTopic;

#[test]
fn change_topic_serializes_as_snake_case() {
    let cases = [
        (ChangeTopic::Lists, "\"lists\""),
        (ChangeTopic::Items, "\"items\""),
        (ChangeTopic::Tags, "\"tags\""),
        (ChangeTopic::Containers, "\"containers\""),
        (ChangeTopic::Settings, "\"settings\""),
        (ChangeTopic::Comments, "\"comments\""),
        (ChangeTopic::Relations, "\"relations\""),
        (ChangeTopic::TimeEntries, "\"time_entries\""),
        (ChangeTopic::Templates, "\"templates\""),
        (ChangeTopic::Locations, "\"locations\""),
    ];

    for (topic, expected) in cases {
        assert_eq!(serde_json::to_string(&topic).unwrap(), expected);
        assert_eq!(
            serde_json::from_str::<ChangeTopic>(expected).unwrap(),
            topic
        );
    }
}

#[test]
fn change_topic_all_lists_every_variant_once() {
    assert_eq!(ChangeTopic::ALL.len(), 10);
    for topic in ChangeTopic::ALL {
        assert_eq!(
            ChangeTopic::ALL.iter().filter(|t| **t == topic).count(),
            1,
            "{topic:?} must appear exactly once in ChangeTopic::ALL"
        );
    }
}
