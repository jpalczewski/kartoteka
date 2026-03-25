use kartoteka_shared::Tag;
use std::collections::HashMap;

/// A tag with its children, used for rendering tag trees.
#[derive(Clone, Debug)]
pub struct TagNode {
    pub tag: Tag,
    pub children: Vec<TagNode>,
}

/// Build a tree of TagNodes from a flat Vec<Tag>.
/// Returns only root nodes (tags with no parent or whose parent is not in the list).
/// Children are sorted alphabetically by name within each parent.
pub fn build_tag_tree(tags: &[Tag]) -> Vec<TagNode> {
    let tag_ids: std::collections::HashSet<&str> = tags.iter().map(|t| t.id.as_str()).collect();

    // Group children by parent_tag_id
    let mut children_map: HashMap<&str, Vec<&Tag>> = HashMap::new();
    let mut roots: Vec<&Tag> = Vec::new();

    for tag in tags {
        match &tag.parent_tag_id {
            Some(pid) if tag_ids.contains(pid.as_str()) => {
                children_map.entry(pid.as_str()).or_default().push(tag);
            }
            _ => roots.push(tag),
        }
    }

    // Sort roots alphabetically
    roots.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Sort children within each parent
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    fn build_subtree<'a>(tag: &'a Tag, children_map: &HashMap<&str, Vec<&'a Tag>>) -> TagNode {
        let children = children_map
            .get(tag.id.as_str())
            .map(|kids| kids.iter().map(|t| build_subtree(t, children_map)).collect())
            .unwrap_or_default();
        TagNode {
            tag: tag.clone(),
            children,
        }
    }

    roots.iter().map(|t| build_subtree(t, &children_map)).collect()
}

/// Walk ancestors of a tag (from tag up to root). Returns list from root to tag.
pub fn build_breadcrumb(tags: &[Tag], tag_id: &str) -> Vec<Tag> {
    let tag_map: HashMap<&str, &Tag> = tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut path = Vec::new();
    let mut current_id = Some(tag_id);

    let mut visited = std::collections::HashSet::new();
    while let Some(id) = current_id {
        if !visited.insert(id.to_string()) {
            break; // cycle protection
        }
        if let Some(tag) = tag_map.get(id) {
            path.push((*tag).clone());
            current_id = tag.parent_tag_id.as_deref();
        } else {
            break;
        }
    }

    path.reverse(); // root first
    path
}
