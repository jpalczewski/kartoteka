use std::collections::HashMap;

use kartoteka_shared::SetItemPlacementRequest;

use super::reorder::reorder_ids;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DndState {
    pub dragged_id: Option<String>,
    pub hovered_target: Option<DropTarget>,
}

impl DndState {
    pub fn is_active(&self) -> bool {
        self.dragged_id.is_some()
    }

    pub fn dragged_id(&self) -> Option<&str> {
        self.dragged_id.as_deref()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropTarget {
    Before(String),
    End,
}

impl DropTarget {
    pub fn before(id: impl Into<String>) -> Self {
        Self::Before(id.into())
    }

    pub const fn end() -> Self {
        Self::End
    }

    pub fn before_id(&self) -> Option<&str> {
        match self {
            Self::Before(id) => Some(id.as_str()),
            Self::End => None,
        }
    }

    pub fn is_end(&self) -> bool {
        matches!(self, Self::End)
    }
}

pub fn begin_drag(state: &mut DndState, dragged_id: impl Into<String>) {
    state.dragged_id = Some(dragged_id.into());
    state.hovered_target = None;
}

pub fn set_hovered_target(state: &mut DndState, target: DropTarget) {
    state.hovered_target = Some(target);
}

pub fn clear_dnd_state(state: &mut DndState) {
    state.dragged_id = None;
    state.hovered_target = None;
}

pub fn is_dragged_id(state: &DndState, dragged_id: &str) -> bool {
    state.dragged_id() == Some(dragged_id)
}

pub fn is_hovered_target(state: &DndState, target: &DropTarget) -> bool {
    state.hovered_target.as_ref() == Some(target)
}

pub fn reorder_ids_for_target(
    ids: &[String],
    dragged_id: &str,
    target: &DropTarget,
) -> Option<Vec<String>> {
    reorder_ids(ids, dragged_id, target.before_id())
}

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemDropPlan {
    Reorder {
        list_id: String,
        item_ids: Vec<String>,
    },
    Move {
        item_id: String,
        request: SetItemPlacementRequest,
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

pub fn build_item_drop_plan(
    item_ids_by_list: &HashMap<String, Vec<String>>,
    dragged_item: &DraggedItem,
    target: &ItemDropTarget,
) -> Option<ItemDropPlan> {
    let source_ids = item_ids_by_list.get(&dragged_item.source_list_id)?;
    let target_list_id = target.list_id().to_string();

    if dragged_item.source_list_id == target_list_id {
        let next_ids = reorder_ids(source_ids, &dragged_item.item_id, target.before_item_id())?;
        return Some(ItemDropPlan::Reorder {
            list_id: target_list_id,
            item_ids: next_ids,
        });
    }

    let source_item_ids: Vec<String> = source_ids
        .iter()
        .filter(|item_id| item_id.as_str() != dragged_item.item_id.as_str())
        .cloned()
        .collect();
    let target_ids = item_ids_by_list.get(target.list_id())?;
    let mut target_with_dragged = target_ids.clone();
    target_with_dragged.push(dragged_item.item_id.clone());
    let target_item_ids = reorder_ids(
        &target_with_dragged,
        &dragged_item.item_id,
        target.before_item_id(),
    )
    .unwrap_or(target_with_dragged);

    Some(ItemDropPlan::Move {
        item_id: dragged_item.item_id.clone(),
        request: SetItemPlacementRequest {
            source_list_id: dragged_item.source_list_id.clone(),
            target_list_id,
            source_item_ids,
            target_item_ids,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn begin_drag_sets_dragged_id_and_clears_hovered_target() {
        let mut state = DndState {
            dragged_id: Some("old".into()),
            hovered_target: Some(DropTarget::end()),
        };

        begin_drag(&mut state, "new");

        assert_eq!(state.dragged_id(), Some("new"));
        assert_eq!(state.hovered_target, None);
    }

    #[test]
    fn clear_dnd_state_resets_drag_operation() {
        let mut state = DndState {
            dragged_id: Some("item-1".into()),
            hovered_target: Some(DropTarget::before("item-2")),
        };

        clear_dnd_state(&mut state);

        assert_eq!(state, DndState::default());
    }

    #[test]
    fn is_hovered_target_matches_before_target() {
        let state = DndState {
            dragged_id: Some("item-1".into()),
            hovered_target: Some(DropTarget::before("item-2")),
        };

        assert!(is_hovered_target(&state, &DropTarget::before("item-2")));
        assert!(!is_hovered_target(&state, &DropTarget::before("item-3")));
    }

    #[test]
    fn reorder_ids_for_target_moves_item_before_before_target() {
        let ids = vec!["a".into(), "b".into(), "c".into()];

        let next_ids =
            reorder_ids_for_target(&ids, "c", &DropTarget::before("b")).expect("reordered ids");

        assert_eq!(next_ids, vec!["a", "c", "b"]);
    }

    #[test]
    fn reorder_ids_for_target_moves_item_to_end() {
        let ids = vec!["a".into(), "b".into(), "c".into()];

        let next_ids =
            reorder_ids_for_target(&ids, "a", &DropTarget::end()).expect("reordered ids");

        assert_eq!(next_ids, vec!["b", "c", "a"]);
    }

    #[test]
    fn build_item_drop_plan_reorders_within_same_list() {
        let item_ids_by_list =
            HashMap::from([("list-1".into(), vec!["a".into(), "b".into(), "c".into()])]);
        let dragged_item = DraggedItem {
            item_id: "c".into(),
            source_list_id: "list-1".into(),
        };

        let plan = build_item_drop_plan(
            &item_ids_by_list,
            &dragged_item,
            &ItemDropTarget::before("list-1", "b"),
        )
        .expect("drop plan");

        assert_eq!(
            plan,
            ItemDropPlan::Reorder {
                list_id: "list-1".into(),
                item_ids: vec!["a".into(), "c".into(), "b".into()],
            }
        );
    }

    #[test]
    fn build_item_drop_plan_moves_between_lists() {
        let item_ids_by_list = HashMap::from([
            ("main".into(), vec!["a".into(), "b".into()]),
            ("sub".into(), vec!["c".into()]),
        ]);
        let dragged_item = DraggedItem {
            item_id: "b".into(),
            source_list_id: "main".into(),
        };

        let plan = build_item_drop_plan(
            &item_ids_by_list,
            &dragged_item,
            &ItemDropTarget::before("sub", "c"),
        )
        .expect("drop plan");

        assert_eq!(
            plan,
            ItemDropPlan::Move {
                item_id: "b".into(),
                request: SetItemPlacementRequest {
                    source_list_id: "main".into(),
                    target_list_id: "sub".into(),
                    source_item_ids: vec!["a".into()],
                    target_item_ids: vec!["b".into(), "c".into()],
                },
            }
        );
    }
}
