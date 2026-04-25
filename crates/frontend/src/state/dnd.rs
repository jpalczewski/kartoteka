//! Pure state reducers for drag-and-drop. No Leptos, no web-sys.
//!
//! Two state machines:
//! - [`DndState`] for lists and containers (one id at a time).
//! - [`ItemDndState`] for items, which also need to know their source list.

use std::collections::HashMap;

use super::reorder::reorder_ids;

// ── Generic list / container DnD ─────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    List,
    Container,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedEntity {
    pub kind: EntityKind,
    pub id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DndState {
    pub dragged: Option<DraggedEntity>,
    pub hovered_target: Option<DropTarget>,
}

impl DndState {
    pub fn is_active(&self) -> bool {
        self.dragged.is_some()
    }

    pub fn dragged_id(&self) -> Option<&str> {
        self.dragged.as_ref().map(|d| d.id.as_str())
    }

    pub fn dragged_kind(&self) -> Option<EntityKind> {
        self.dragged.as_ref().map(|d| d.kind)
    }
}

/// Where a drag can land. `Before`/`End` are reorder markers between cards;
/// `Nest` is the card body itself (nesting); `Detach` is a top-level zone that
/// moves the dragged entity up one level in the hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropTarget {
    Before(String),
    End,
    Nest(String),
    Detach,
}

impl DropTarget {
    pub fn before(id: impl Into<String>) -> Self {
        Self::Before(id.into())
    }

    pub const fn end() -> Self {
        Self::End
    }

    pub fn nest(id: impl Into<String>) -> Self {
        Self::Nest(id.into())
    }

    pub fn before_id(&self) -> Option<&str> {
        match self {
            Self::Before(id) => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn nest_id(&self) -> Option<&str> {
        match self {
            Self::Nest(id) => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn is_end(&self) -> bool {
        matches!(self, Self::End)
    }

    pub fn is_detach(&self) -> bool {
        matches!(self, Self::Detach)
    }
}

pub fn begin_drag(state: &mut DndState, kind: EntityKind, dragged_id: impl Into<String>) {
    state.dragged = Some(DraggedEntity {
        kind,
        id: dragged_id.into(),
    });
    state.hovered_target = None;
}

pub fn set_hovered_target(state: &mut DndState, target: DropTarget) {
    state.hovered_target = Some(target);
}

pub fn clear_dnd_state(state: &mut DndState) {
    state.dragged = None;
    state.hovered_target = None;
}

pub fn is_dragged_id(state: &DndState, dragged_id: &str) -> bool {
    state.dragged_id() == Some(dragged_id)
}

pub fn is_hovered_target(state: &DndState, target: &DropTarget) -> bool {
    state.hovered_target.as_ref() == Some(target)
}

/// Compute a new id order for a reorder drop (Before/End). Returns None if
/// the target is not a reorder marker or the order would not change.
pub fn reorder_ids_for_target(
    ids: &[String],
    dragged_id: &str,
    target: &DropTarget,
) -> Option<Vec<String>> {
    match target {
        DropTarget::Before(_) | DropTarget::End => reorder_ids(ids, dragged_id, target.before_id()),
        _ => None,
    }
}

// ── Item DnD (cross-list move capable) ────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraggedItem {
    pub item_id: String,
    pub source_list_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemDndState {
    pub dragged_item: Option<DraggedItem>,
    pub hovered_target: Option<ItemDropTarget>,
}

impl ItemDndState {
    pub fn is_active(&self) -> bool {
        self.dragged_item.is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemDropTarget {
    Before {
        list_id: String,
        before_item_id: String,
    },
    End {
        list_id: String,
    },
}

impl ItemDropTarget {
    pub fn before(list_id: impl Into<String>, before_item_id: impl Into<String>) -> Self {
        Self::Before {
            list_id: list_id.into(),
            before_item_id: before_item_id.into(),
        }
    }

    pub fn end(list_id: impl Into<String>) -> Self {
        Self::End {
            list_id: list_id.into(),
        }
    }

    pub fn list_id(&self) -> &str {
        match self {
            Self::Before { list_id, .. } | Self::End { list_id } => list_id.as_str(),
        }
    }

    pub fn before_item_id(&self) -> Option<&str> {
        match self {
            Self::Before { before_item_id, .. } => Some(before_item_id.as_str()),
            Self::End { .. } => None,
        }
    }
}

/// Result of an item drop decision. Pages translate this into a server call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemDropPlan {
    /// Same-list reorder: send the new full id order.
    Reorder {
        list_id: String,
        item_ids: Vec<String>,
    },
    /// Cross-list move: move `item_id` to `target_list_id`, inserted at the
    /// position indicated by `before_item_id` (None = append).
    Move {
        item_id: String,
        source_list_id: String,
        target_list_id: String,
        before_item_id: Option<String>,
    },
}

pub fn begin_item_drag(state: &mut ItemDndState, dragged_item: DraggedItem) {
    state.dragged_item = Some(dragged_item);
    state.hovered_target = None;
}

pub fn set_hovered_item_target(state: &mut ItemDndState, target: ItemDropTarget) {
    state.hovered_target = Some(target);
}

pub fn clear_item_dnd_state(state: &mut ItemDndState) {
    state.dragged_item = None;
    state.hovered_target = None;
}

pub fn is_dragged_item(state: &ItemDndState, dragged_item: &DraggedItem) -> bool {
    state.dragged_item.as_ref() == Some(dragged_item)
}

pub fn is_hovered_item_target(state: &ItemDndState, target: &ItemDropTarget) -> bool {
    state.hovered_target.as_ref() == Some(target)
}

/// Build a drop plan given the current id layout (list_id → ordered item ids),
/// the dragged item, and the hovered target. Returns None if the plan is a no-op.
pub fn build_item_drop_plan(
    item_ids_by_list: &HashMap<String, Vec<String>>,
    dragged_item: &DraggedItem,
    target: &ItemDropTarget,
) -> Option<ItemDropPlan> {
    let target_list_id = target.list_id().to_string();

    if dragged_item.source_list_id == target_list_id {
        let source_ids = item_ids_by_list.get(&dragged_item.source_list_id)?;
        let next_ids = reorder_ids(source_ids, &dragged_item.item_id, target.before_item_id())?;
        return Some(ItemDropPlan::Reorder {
            list_id: target_list_id,
            item_ids: next_ids,
        });
    }

    Some(ItemDropPlan::Move {
        item_id: dragged_item.item_id.clone(),
        source_list_id: dragged_item.source_list_id.clone(),
        target_list_id,
        before_item_id: target.before_item_id().map(str::to_string),
    })
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(kind: EntityKind, id: &str) -> DraggedEntity {
        DraggedEntity {
            kind,
            id: id.to_string(),
        }
    }

    #[test]
    fn begin_drag_sets_id_and_clears_target() {
        let mut st = DndState {
            dragged: Some(ent(EntityKind::List, "old")),
            hovered_target: Some(DropTarget::end()),
        };
        begin_drag(&mut st, EntityKind::Container, "new");
        assert_eq!(st.dragged_id(), Some("new"));
        assert_eq!(st.dragged_kind(), Some(EntityKind::Container));
        assert_eq!(st.hovered_target, None);
    }

    #[test]
    fn clear_dnd_state_resets() {
        let mut st = DndState {
            dragged: Some(ent(EntityKind::List, "a")),
            hovered_target: Some(DropTarget::before("b")),
        };
        clear_dnd_state(&mut st);
        assert_eq!(st, DndState::default());
    }

    #[test]
    fn hovered_target_match_by_variant() {
        let st = DndState {
            dragged: Some(ent(EntityKind::List, "a")),
            hovered_target: Some(DropTarget::before("b")),
        };
        assert!(is_hovered_target(&st, &DropTarget::before("b")));
        assert!(!is_hovered_target(&st, &DropTarget::before("c")));
        assert!(!is_hovered_target(&st, &DropTarget::nest("b")));
    }

    #[test]
    fn nest_and_detach_variants_round_trip() {
        assert_eq!(DropTarget::nest("x").nest_id(), Some("x"));
        assert!(DropTarget::Detach.is_detach());
        assert!(DropTarget::nest("x").before_id().is_none());
    }

    #[test]
    fn reorder_ids_for_target_handles_reorder_kinds_only() {
        let ids = vec!["a".into(), "b".into(), "c".into()];
        let r = reorder_ids_for_target(&ids, "c", &DropTarget::before("b")).unwrap();
        assert_eq!(r, vec!["a", "c", "b"]);
        let r2 = reorder_ids_for_target(&ids, "a", &DropTarget::end()).unwrap();
        assert_eq!(r2, vec!["b", "c", "a"]);
        assert!(reorder_ids_for_target(&ids, "a", &DropTarget::nest("b")).is_none());
        assert!(reorder_ids_for_target(&ids, "a", &DropTarget::Detach).is_none());
    }

    #[test]
    fn item_plan_reorders_in_same_list() {
        let ids = HashMap::from([("L".to_string(), vec!["a".into(), "b".into(), "c".into()])]);
        let dragged = DraggedItem {
            item_id: "c".into(),
            source_list_id: "L".into(),
        };
        let plan = build_item_drop_plan(&ids, &dragged, &ItemDropTarget::before("L", "b")).unwrap();
        assert_eq!(
            plan,
            ItemDropPlan::Reorder {
                list_id: "L".into(),
                item_ids: vec!["a".into(), "c".into(), "b".into()],
            }
        );
    }

    #[test]
    fn item_plan_moves_between_lists() {
        let ids = HashMap::from([
            ("main".to_string(), vec!["a".into(), "b".into()]),
            ("sub".to_string(), vec!["c".into()]),
        ]);
        let dragged = DraggedItem {
            item_id: "b".into(),
            source_list_id: "main".into(),
        };
        let plan =
            build_item_drop_plan(&ids, &dragged, &ItemDropTarget::before("sub", "c")).unwrap();
        assert_eq!(
            plan,
            ItemDropPlan::Move {
                item_id: "b".into(),
                source_list_id: "main".into(),
                target_list_id: "sub".into(),
                before_item_id: Some("c".into()),
            }
        );
    }

    #[test]
    fn item_plan_move_to_end_of_other_list() {
        let ids = HashMap::from([
            ("main".to_string(), vec!["a".into(), "b".into()]),
            ("sub".to_string(), vec![]),
        ]);
        let dragged = DraggedItem {
            item_id: "a".into(),
            source_list_id: "main".into(),
        };
        let plan = build_item_drop_plan(&ids, &dragged, &ItemDropTarget::end("sub")).unwrap();
        assert_eq!(
            plan,
            ItemDropPlan::Move {
                item_id: "a".into(),
                source_list_id: "main".into(),
                target_list_id: "sub".into(),
                before_item_id: None,
            }
        );
    }
}
