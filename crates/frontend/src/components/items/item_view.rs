//! Shared item row: what an item view shows and which actions it offers.
//!
//! Every page that lists items (list, sublists, Today, All, calendar day/week, tag detail,
//! later the Kanban board) renders the same `ItemView` component, configured with an
//! [`ItemViewVariant`] and [`ItemCaps`]. The component itself is built in steps:
//!
//! 1. pure helpers in `kartoteka_shared` (item link, overdue, date chips, quantity label);
//! 2. small parts in `components/items/parts/` (checkbox, title, date chips, quantity,
//!    recurrence/status chips, icon), each reading one keyed store row;
//! 3. `Compact` variant ([`super::item_compact::ItemCompact`]), replacing the per-page rows of
//!    Today, All, calendar and tag detail (done);
//! 4. the `Row` variant ([`super::item_row::ItemRow`]: expandable editor, action menu, drag
//!    handle), replacing the by-value `ItemRow` (done);
//! 5. the list page and sublists on keyed stores with the `Row` variant, rows keyed by id in
//!    [`super::item_list::ItemRowList`] (done).
//!
//! There is no single dispatching component: the two variants take different inputs (`Row`
//! gets move targets, tags and drag state, `Compact` a layout and test ids) and every page
//! uses exactly one of them, so each page renders `ItemRow` or `ItemCompact` directly.
//!
//! Data contract: a view gets `store: Store<ItemRows<T>>` + `id` and reads its item through
//! [`crate::state::item_store::row_item`], so it is created once per item and updates in
//! place (see `state/item_store.rs`). Dates are compared against
//! [`crate::state::user_today::use_today`], never the browser clock. Mutations go through
//! [`crate::state::item_actions::use_item_actions`].

use kartoteka_shared::constants::{
    FEATURE_CHECKLIST, FEATURE_DEADLINES, FEATURE_QUANTITY, FEATURE_TIME_TRACKING,
};

/// Layout of an item view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemViewVariant {
    /// Full list row: expandable editor, quantity stepper, tags, action menu, drag handle.
    #[default]
    Row,
    /// Dense row for cross-list views (Today, All, calendar, tag detail): no editor.
    Compact,
}

/// What an item view shows and lets the user do. Pages build it from the list's features
/// ([`ItemCaps::from_features`]) and switch off what the page does not offer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemCaps {
    /// Completion checkbox (lists with the checklist feature).
    pub checkbox: bool,
    /// Date chips and, in `Row`, the date editor (deadlines feature).
    pub dates: bool,
    /// Quantity label/stepper (quantity feature).
    pub quantity: bool,
    /// Time tracking controls (time_tracking feature).
    pub time: bool,
    /// Item tags (chips, and the tag selector in `Row`).
    pub tags: bool,
    /// Delete action.
    pub delete: bool,
    /// Move-to-list action (`Row` only).
    pub move_to: bool,
    /// Drag handle for reordering (`Row` only).
    pub drag: bool,
    /// Show the owning list's name (cross-list views).
    pub list_name: bool,
}

impl ItemCaps {
    /// Everything off; `Default` is not `const`, so pages build their caps from this.
    pub const NONE: Self = Self {
        checkbox: false,
        dates: false,
        quantity: false,
        time: false,
        tags: false,
        delete: false,
        move_to: false,
        drag: false,
        list_name: false,
    };

    /// Capabilities implied by a list's enabled feature names. Actions (`delete`,
    /// `move_to`, `drag`, `tags`, `list_name`) stay off; pages opt in.
    pub fn from_features<S: AsRef<str>>(features: &[S]) -> Self {
        let has = |name: &str| features.iter().any(|f| f.as_ref() == name);
        Self {
            checkbox: has(FEATURE_CHECKLIST),
            dates: has(FEATURE_DEADLINES),
            quantity: has(FEATURE_QUANTITY),
            time: has(FEATURE_TIME_TRACKING),
            ..Self::NONE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_features_maps_feature_names_and_leaves_actions_off() {
        let caps = ItemCaps::from_features(&["checklist", "deadlines"]);
        assert!(caps.checkbox && caps.dates);
        assert!(!caps.quantity && !caps.time);
        assert!(!caps.delete && !caps.move_to && !caps.drag && !caps.tags && !caps.list_name);
    }

    #[test]
    fn from_features_with_no_features_shows_nothing() {
        assert_eq!(ItemCaps::from_features::<&str>(&[]), ItemCaps::default());
    }
}
