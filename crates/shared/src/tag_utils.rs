use std::collections::HashMap;

use crate::types::Tag;

/// Returns the full ancestor path of a tag joined by `sep` (root → leaf).
/// E.g. with sep `"\\"`: "Hobby\\Ogród\\Warzywa"
pub fn ancestor_path(tag: &Tag, all_tags: &[Tag], sep: &str) -> String {
    let by_id: HashMap<&str, &Tag> = all_tags.iter().map(|t| (t.id.as_str(), t)).collect();
    ancestor_path_with_map(tag, &by_id, sep)
}

/// Builds a map of `tag_id → ancestor_path` for all tags in one pass.
/// Use this when you need paths for many tags — avoids rebuilding the lookup map per tag.
pub fn build_ancestor_map(all_tags: &[Tag], sep: &str) -> HashMap<String, String> {
    let by_id: HashMap<&str, &Tag> = all_tags.iter().map(|t| (t.id.as_str(), t)).collect();
    all_tags
        .iter()
        .map(|tag| (tag.id.clone(), ancestor_path_with_map(tag, &by_id, sep)))
        .collect()
}

fn ancestor_path_with_map<'a>(tag: &'a Tag, by_id: &HashMap<&str, &'a Tag>, sep: &str) -> String {
    let mut parts = vec![tag.name.clone()];
    let mut current = tag;
    while let Some(ref pid) = current.parent_tag_id {
        match by_id.get(pid.as_str()) {
            Some(&parent) => {
                parts.push(parent.name.clone());
                current = parent;
            }
            None => break,
        }
    }
    parts.reverse();
    parts.join(sep)
}
