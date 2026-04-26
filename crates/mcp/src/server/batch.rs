//! Helpers shared by batch-create tools (`create_items`, `create_lists`,
//! `create_containers`).
//!
//! `PositionAllocator` keeps per-scope counters so that newly inserted entities
//! get monotonically increasing positions within each scope, while pre-fetched
//! database "next position" values seed the starting offset.

use std::collections::HashMap;
use std::hash::Hash;

/// Tracks position counters keyed by scope.
///
/// Typical use:
/// 1. Call `set_base` for each scope with the value returned by the database
///    `next_position` query.
/// 2. Call `allocate` per inserted entity to receive a unique, monotonic
///    position.
pub struct PositionAllocator<K> {
    base: HashMap<K, i64>,
    offsets: HashMap<K, i64>,
}

impl<K: Eq + Hash + Clone> Default for PositionAllocator<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Eq + Hash + Clone> PositionAllocator<K> {
    pub fn new() -> Self {
        Self {
            base: HashMap::new(),
            offsets: HashMap::new(),
        }
    }

    pub fn set_base(&mut self, key: K, base: i64) {
        self.base.insert(key, base);
    }

    pub fn allocate(&mut self, key: K) -> i64 {
        let base = self.base.get(&key).copied().unwrap_or(0);
        let offset = self.offsets.entry(key).or_insert(0);
        let pos = base + *offset;
        *offset += 1;
        pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_without_base_starts_at_zero() {
        let mut a = PositionAllocator::<&'static str>::new();
        assert_eq!(a.allocate("x"), 0);
        assert_eq!(a.allocate("x"), 1);
        assert_eq!(a.allocate("x"), 2);
    }

    #[test]
    fn set_base_seeds_offset() {
        let mut a = PositionAllocator::new();
        a.set_base("x", 10);
        assert_eq!(a.allocate("x"), 10);
        assert_eq!(a.allocate("x"), 11);
    }

    #[test]
    fn scopes_are_independent() {
        let mut a = PositionAllocator::new();
        a.set_base("a", 5);
        a.set_base("b", 100);
        assert_eq!(a.allocate("a"), 5);
        assert_eq!(a.allocate("b"), 100);
        assert_eq!(a.allocate("a"), 6);
        assert_eq!(a.allocate("b"), 101);
        assert_eq!(a.allocate("a"), 7);
    }

    #[test]
    fn tuple_key_works_for_lists_scope() {
        // Mirrors lists scope: (container_id, parent_list_id)
        let mut a: PositionAllocator<(Option<String>, Option<String>)> = PositionAllocator::new();
        a.set_base((Some("c1".into()), None), 3);
        let k1 = (Some("c1".to_string()), None);
        let k2 = (None, Some("p1".to_string()));
        assert_eq!(a.allocate(k1.clone()), 3);
        assert_eq!(a.allocate(k2.clone()), 0);
        assert_eq!(a.allocate(k1), 4);
        assert_eq!(a.allocate(k2), 1);
    }
}
