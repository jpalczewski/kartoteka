//! Small parts of an item view. Each takes the row's current item as
//! `Signal<Option<Item>>` (see [`crate::state::item_store::item_signal`]) and renders nothing
//! while it is `None`. Parts read the item through closures, so a change to one field updates
//! only the DOM that shows it.

mod checkbox;
mod date_chips;
mod estimate_chip;
mod icon;
mod quantity_badge;
mod recurrence_chip;
mod status_chip;
mod title;

pub use checkbox::ItemCheckbox;
pub use date_chips::DateChips;
pub use estimate_chip::EstimateChip;
pub use icon::ItemIcon;
pub use quantity_badge::QuantityBadge;
pub use recurrence_chip::RecurrenceChip;
pub use status_chip::StatusChip;
pub use title::ItemTitle;
