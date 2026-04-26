use std::collections::HashMap;

/// Maps caller-chosen string labels to real UUIDs during a single batch operation.
///
/// Allows a batch to reference an entity created earlier in the same batch via
/// `*_ref` fields instead of a real UUID. References are forward-only: a ref can
/// only point to an entity registered before the current position in the batch.
pub struct RefResolver {
    map: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum RefError {
    #[error("duplicate client_ref '{0}'")]
    Duplicate(String),
    #[error("unknown client_ref '{0}'")]
    Unknown(String),
    #[error("cannot set both id '{id}' and ref '{reference}' for the same field")]
    Conflicting { id: String, reference: String },
    #[error("required field has neither an id nor a ref")]
    MissingRequired,
}

impl RefResolver {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Register a newly-created entity under its `client_ref` label (no-op when `None`).
    pub fn register(&mut self, client_ref: Option<&str>, real_id: &str) -> Result<(), RefError> {
        if let Some(label) = client_ref {
            if self.map.contains_key(label) {
                return Err(RefError::Duplicate(label.to_owned()));
            }
            self.map.insert(label.to_owned(), real_id.to_owned());
        }
        Ok(())
    }

    /// Resolve a reference to the real UUID registered for it.
    pub fn resolve(&self, reference: &str) -> Result<&str, RefError> {
        self.map
            .get(reference)
            .map(String::as_str)
            .ok_or_else(|| RefError::Unknown(reference.to_owned()))
    }

    /// Pick the effective ID from an (id, ref) pair.
    ///
    /// - Both set → `RefError::Conflicting`
    /// - Only `id` set → returns `Some(id)` (not resolved, already a real UUID)
    /// - Only `reference` set → resolves and returns `Some(resolved_id)`
    /// - Neither set and `required` → `RefError::MissingRequired`
    /// - Neither set and optional → returns `None`
    pub fn pick<'a>(
        &'a self,
        id: Option<&'a str>,
        reference: Option<&'a str>,
        required: bool,
    ) -> Result<Option<&'a str>, RefError> {
        match (id, reference) {
            (Some(id), Some(reference)) => Err(RefError::Conflicting {
                id: id.to_owned(),
                reference: reference.to_owned(),
            }),
            (Some(id), None) => Ok(Some(id)),
            (None, Some(reference)) => Ok(Some(self.resolve(reference)?)),
            (None, None) if required => Err(RefError::MissingRequired),
            (None, None) => Ok(None),
        }
    }
}

impl Default for RefResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve() {
        let mut r = RefResolver::new();
        r.register(Some("proj"), "uuid-1").unwrap();
        assert_eq!(r.resolve("proj").unwrap(), "uuid-1");
    }

    #[test]
    fn register_none_is_noop() {
        let mut r = RefResolver::new();
        r.register(None, "uuid-1").unwrap();
        assert!(r.map.is_empty());
    }

    #[test]
    fn duplicate_ref_is_error() {
        let mut r = RefResolver::new();
        r.register(Some("x"), "uuid-1").unwrap();
        assert_eq!(
            r.register(Some("x"), "uuid-2").unwrap_err(),
            RefError::Duplicate("x".into())
        );
    }

    #[test]
    fn unknown_ref_is_error() {
        let r = RefResolver::new();
        assert_eq!(
            r.resolve("missing").unwrap_err(),
            RefError::Unknown("missing".into())
        );
    }

    #[test]
    fn pick_id_only() {
        let r = RefResolver::new();
        assert_eq!(
            r.pick(Some("real-id"), None, false).unwrap(),
            Some("real-id")
        );
    }

    #[test]
    fn pick_ref_only_resolves() {
        let mut r = RefResolver::new();
        r.register(Some("proj"), "uuid-42").unwrap();
        assert_eq!(r.pick(None, Some("proj"), false).unwrap(), Some("uuid-42"));
    }

    #[test]
    fn pick_both_is_conflicting() {
        let r = RefResolver::new();
        assert!(matches!(
            r.pick(Some("id"), Some("ref"), false).unwrap_err(),
            RefError::Conflicting { .. }
        ));
    }

    #[test]
    fn pick_neither_optional_returns_none() {
        let r = RefResolver::new();
        assert_eq!(r.pick(None, None, false).unwrap(), None);
    }

    #[test]
    fn pick_neither_required_is_error() {
        let r = RefResolver::new();
        assert_eq!(
            r.pick(None, None, true).unwrap_err(),
            RefError::MissingRequired
        );
    }

    #[test]
    fn forward_only_works_in_sequence() {
        let mut r = RefResolver::new();
        // A is created first
        let id_a = "uuid-a";
        r.register(Some("a"), id_a).unwrap();
        // B references A (forward-only is maintained by the caller ordering the loop)
        let resolved = r.pick(None, Some("a"), false).unwrap();
        assert_eq!(resolved, Some(id_a));
        // After creating B, register it
        r.register(Some("b"), "uuid-b").unwrap();
        // C references B
        assert_eq!(r.pick(None, Some("b"), false).unwrap(), Some("uuid-b"));
    }

    #[test]
    fn backward_ref_to_unregistered_fails() {
        let r = RefResolver::new();
        // "future" was never registered — behaves as unknown
        assert_eq!(
            r.pick(None, Some("future"), false).unwrap_err(),
            RefError::Unknown("future".into())
        );
    }
}
