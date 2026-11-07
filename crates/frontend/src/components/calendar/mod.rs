pub mod week_view;

use crate::components::items::item_compact::CompactTestIds;
use crate::components::items::item_view::ItemCaps;

/// Calendar rows: completion checkbox and delete; the date is the day they sit under.
pub const ROW_CAPS: ItemCaps = ItemCaps {
    checkbox: true,
    delete: true,
    ..ItemCaps::NONE
};

pub const ROW_TESTIDS: CompactTestIds = CompactTestIds {
    row: Some("calendar-item-row"),
    toggle: None,
    title: None,
    description: None,
};
