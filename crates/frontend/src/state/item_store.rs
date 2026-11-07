//! Keyed, per-row reactive item data for item list views.
//!
//! A page patches every refetch of its item resource into an [`ItemRows`] store,
//! keyed by item id. Rows whose data did not change are not notified, changed rows
//! update in place, and a `<For>` keyed by the same id keeps each row component,
//! with its local UI state (an expanded editor, focus), alive across refetches and
//! live updates.
//!
//! Pattern for a page:
//! 1. `let rows = Store::new(ItemRows::<DateItem>::default());`
//! 2. Inside the `<Transition>`, a render closure reads the resource and calls
//!    `rows.rows().patch(...)`. It runs before the list in the same render pass,
//!    on the server and in the browser, so SSR output and hydration agree.
//! 3. Structure (which ids, grouped how, filtered how) is derived from the store
//!    with [`ItemRows::tracked_items`], so it follows the patched data.
//! 4. `<For each=... key=|id| id.clone()>` renders a row per id; the row reads its
//!    data with [`row_item`].

use kartoteka_shared::types::{DateItem, Item};
use leptos::prelude::*;
use reactive_stores::{KeyMap, PatchField, Store, StorePath};

/// A list entry patched as a whole: replaced, and its row notified, only when it changed.
#[derive(Clone, Debug, PartialEq)]
pub struct Row<T>(pub T);

impl<T: PartialEq> PatchField for Row<T> {
    fn patch_field(
        &mut self,
        new: Self,
        path: &StorePath,
        notify: &mut dyn FnMut(&StorePath),
        _keys: Option<&KeyMap>,
    ) {
        if new != *self {
            *self = new;
            notify(path);
        }
    }
}

/// An entry of an item list view: anything that carries an [`Item`].
pub trait ItemEntry: Clone + PartialEq + Send + Sync + 'static {
    fn item(&self) -> &Item;

    /// Type of the list the item belongs to, when the entry knows it (cross-list views).
    fn list_type(&self) -> Option<&str> {
        None
    }
}

impl ItemEntry for Item {
    fn item(&self) -> &Item {
        self
    }
}

impl ItemEntry for DateItem {
    fn item(&self) -> &Item {
        &self.item
    }

    fn list_type(&self) -> Option<&str> {
        Some(&self.list_type)
    }
}

/// The rows of one item list view, keyed by item id.
#[derive(Store, Debug)]
pub struct ItemRows<T: ItemEntry> {
    #[store(key: String = |row| row.0.item().id.clone())]
    pub rows: Vec<Row<T>>,
}

impl<T: ItemEntry> Default for ItemRows<T> {
    fn default() -> Self {
        Self { rows: Vec::new() }
    }
}

/// Replace the rows with `entries`, notifying only rows that changed (and the list
/// itself when ids were added, removed or reordered).
pub fn patch_rows<T: ItemEntry>(store: Store<ItemRows<T>>, entries: impl IntoIterator<Item = T>) {
    store.rows().patch(entries.into_iter().map(Row).collect());
}

/// Every entry, reading each row reactively: a derived structure (grouping,
/// filtering, counts) recomputes when any row's data changes, not only when the
/// set of ids does.
pub fn tracked_items<T: ItemEntry>(store: Store<ItemRows<T>>) -> Vec<T> {
    store
        .rows()
        .into_iter()
        .filter_map(|row| row.try_get().map(|row| row.0))
        .collect()
}

/// The current data of the row with `id`; `None` once the row has been patched away
/// (the keyed `<For>` drops the row right after).
pub fn row_item<T: ItemEntry>(store: Store<ItemRows<T>>, id: String) -> Memo<Option<T>> {
    Memo::new(move |_| store.rows().at_key(id.clone()).try_get().map(|row| row.0))
}

/// The row's item as a plain `Signal<Option<Item>>`, which is what the item parts in
/// `components/items/parts/` take. `None` once the row is gone. Use it with a [`row_item`]
/// memo, not a whole-store read, so only that row's updates reach the part.
pub fn item_signal<T: ItemEntry>(row: Memo<Option<T>>) -> Signal<Option<Item>> {
    Signal::derive(move || row.get().map(|entry| entry.item().clone()))
}
