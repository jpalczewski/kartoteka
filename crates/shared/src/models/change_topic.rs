use serde::{Deserialize, Serialize};

/// A topic of user-scoped data that can change on the server.
///
/// The frontend uses it to invalidate the caches (Leptos `Resource`s) that load
/// that data, and server-sent events will use it to announce which data changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeTopic {
    Lists,
    Items,
    Tags,
    Containers,
    Settings,
    Comments,
    Relations,
    TimeEntries,
    Templates,
    Locations,
}

impl ChangeTopic {
    /// Every topic, in declaration order.
    pub const ALL: [ChangeTopic; 10] = [
        ChangeTopic::Lists,
        ChangeTopic::Items,
        ChangeTopic::Tags,
        ChangeTopic::Containers,
        ChangeTopic::Settings,
        ChangeTopic::Comments,
        ChangeTopic::Relations,
        ChangeTopic::TimeEntries,
        ChangeTopic::Templates,
        ChangeTopic::Locations,
    ];
}
