//! The list page's copy of its data, split into small pieces.
//!
//! One resource (`get_list_data`) feeds the whole page and refetches on every live update.
//! The page must not rebuild itself from that resource, or open editors, focus and typed text
//! are lost. Instead [`ListPageState::patch`] runs as the first child of the page's
//! `<Transition>`, on the server and in the browser, and copies the data into
//!
//! * keyed item stores (`Store<ItemRows<Item>>`), one for the list and one per sublist, whose
//!   rows are notified only when they changed;
//! * signals that are written only when their value changed.
//!
//! Everything below reads these, through `Memo`s where it needs one small field, so a refetch
//! updates exactly what changed.

use std::collections::HashMap;

use kartoteka_shared::FEATURE_DEADLINES;
use kartoteka_shared::types::{CommentsPayload, Item, ItemTagLink, List, ListData, Tag};
use leptos::prelude::*;
use reactive_stores::Store;

use crate::components::lists::sublist_section::SublistMeta;
use crate::state::item_store::{ItemRows, patch_rows};

/// What the page header needs to know about the list.
#[derive(Clone, Debug, PartialEq)]
pub struct ListMeta {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub pinned: bool,
    pub archived: bool,
    pub container_id: Option<String>,
    pub parent_list_id: Option<String>,
    pub container_name: Option<String>,
    pub parent_list_name: Option<String>,
    pub created_at_local: String,
    pub features: Vec<String>,
    pub deadlines_config: serde_json::Value,
}

fn feature_names(list: &List) -> Vec<String> {
    list.features
        .iter()
        .map(|f| f.feature_name.clone())
        .collect()
}

fn set_if_changed<T: PartialEq + Send + Sync + 'static>(signal: RwSignal<T>, new: T) {
    if signal.with_untracked(|current| *current != new) {
        signal.set(new);
    }
}

#[derive(Clone, Copy)]
pub struct ListPageState {
    /// True once a fetch of the list data has been patched in (and has not failed since).
    pub loaded: RwSignal<bool>,
    pub meta: RwSignal<Option<ListMeta>>,
    pub sublists: RwSignal<Vec<SublistMeta>>,
    /// The list's own items, in the order the server sent them.
    pub items: Store<ItemRows<Item>>,
    pub all_tags: RwSignal<Vec<Tag>>,
    pub item_tag_links: RwSignal<Vec<ItemTagLink>>,
    pub list_tag_ids: RwSignal<Vec<String>>,
    /// Today in the user's timezone, as `YYYY-MM-DD`.
    pub today_date: RwSignal<String>,
    pub comments: RwSignal<CommentsPayload>,
    /// `(id, name)` of every list, for the move menus.
    pub all_lists: RwSignal<Vec<(String, String)>>,
    sublist_stores: StoredValue<HashMap<String, Store<ItemRows<Item>>>>,
    /// The stores outlive the render closure that patches them.
    owner: StoredValue<Owner>,
}

impl ListPageState {
    /// Call in the page component body: the stores belong to the page, not to a closure.
    pub fn new() -> Self {
        Self {
            loaded: RwSignal::new(false),
            meta: RwSignal::new(None),
            sublists: RwSignal::new(Vec::new()),
            items: Store::new(ItemRows::default()),
            all_tags: RwSignal::new(Vec::new()),
            item_tag_links: RwSignal::new(Vec::new()),
            list_tag_ids: RwSignal::new(Vec::new()),
            today_date: RwSignal::new(String::new()),
            comments: RwSignal::new(CommentsPayload {
                comments: Vec::new(),
                current_user_id: String::new(),
            }),
            all_lists: RwSignal::new(Vec::new()),
            sublist_stores: StoredValue::new(HashMap::new()),
            owner: StoredValue::new(Owner::current().expect("ListPageState needs an owner")),
        }
    }

    /// One field of the list's metadata as a memo: dependents re-run only when it changes.
    /// `T::default()` until the first fetch.
    pub fn field<T>(&self, read: impl Fn(&ListMeta) -> T + Send + Sync + 'static) -> Memo<T>
    where
        T: Default + PartialEq + Send + Sync + 'static,
    {
        let meta = self.meta;
        Memo::new(move |_| meta.with(|m| m.as_ref().map(&read).unwrap_or_default()))
    }

    /// The store of a sublist's rows, created on first use.
    pub fn sublist_store(&self, sublist_id: &str) -> Store<ItemRows<Item>> {
        if let Some(store) = self
            .sublist_stores
            .with_value(|stores| stores.get(sublist_id).copied())
        {
            return store;
        }
        let store = self
            .owner
            .get_value()
            .with(|| Store::new(ItemRows::<Item>::default()));
        self.sublist_stores
            .update_value(|stores| _ = stores.insert(sublist_id.to_string(), store));
        store
    }

    /// Copy a fetch of the page data into the stores and signals.
    pub fn patch(&self, data: ListData) {
        let ListData {
            list,
            items,
            sublists,
            created_at_local,
            item_tag_links,
            all_tags,
            list_tag_ids,
            today_date,
            container_name,
            parent_list_name,
            mut sublist_items,
            comments,
        } = data;

        let deadlines_config = list
            .features
            .iter()
            .find(|f| f.feature_name == FEATURE_DEADLINES)
            .map_or_else(|| serde_json::json!({}), |f| f.config.clone());
        set_if_changed(
            self.meta,
            Some(ListMeta {
                features: feature_names(&list),
                deadlines_config,
                id: list.id,
                name: list.name,
                icon: list.icon,
                description: list.description,
                list_type: list.list_type,
                pinned: list.pinned,
                archived: list.archived,
                container_id: list.container_id,
                parent_list_id: list.parent_list_id,
                container_name,
                parent_list_name,
                created_at_local,
            }),
        );

        patch_rows(self.items, items);
        let sublist_ids: Vec<String> = sublists.iter().map(|s| s.id.clone()).collect();
        for sublist in &sublists {
            let rows = sublist_items.remove(&sublist.id).unwrap_or_default();
            patch_rows(self.sublist_store(&sublist.id), rows);
        }
        self.sublist_stores
            .update_value(|stores| stores.retain(|id, _| sublist_ids.contains(id)));
        set_if_changed(
            self.sublists,
            sublists
                .iter()
                .map(|s| SublistMeta {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    features: feature_names(s),
                })
                .collect(),
        );

        set_if_changed(self.loaded, true);
        set_if_changed(self.all_tags, all_tags);
        set_if_changed(self.item_tag_links, item_tag_links);
        set_if_changed(self.list_tag_ids, list_tag_ids);
        set_if_changed(self.today_date, today_date);
        set_if_changed(self.comments, comments);
    }

    /// The last fetch failed: show the error instead of the page.
    pub fn set_failed(&self) {
        set_if_changed(self.loaded, false);
    }

    /// Remember the lists the move menus offer.
    pub fn patch_all_lists(&self, lists: Vec<List>) {
        set_if_changed(
            self.all_lists,
            lists.into_iter().map(|l| (l.id, l.name)).collect(),
        );
    }
}
