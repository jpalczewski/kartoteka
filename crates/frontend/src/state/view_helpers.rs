use std::collections::{HashMap, HashSet};

use kartoteka_shared::{Container, List, Tag};

pub type BreadcrumbCrumb = (String, String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TagFilterOption {
    pub id: String,
    pub label: String,
    pub color: String,
}

pub fn build_tag_breadcrumb(tags: &[Tag], tag_id: &str) -> Vec<Tag> {
    let tag_map: HashMap<&str, &Tag> = tags.iter().map(|tag| (tag.id.as_str(), tag)).collect();
    let mut path = Vec::new();
    let mut current_id = Some(tag_id);
    let mut visited = HashSet::new();

    while let Some(id) = current_id {
        if !visited.insert(id.to_string()) {
            break;
        }
        if let Some(tag) = tag_map.get(id) {
            path.push((*tag).clone());
            current_id = tag.parent_tag_id.as_deref();
        } else {
            break;
        }
    }

    path.reverse();
    path
}

pub fn tag_path_label(tags: &[Tag], tag_id: &str) -> String {
    let breadcrumb = build_tag_breadcrumb(tags, tag_id);
    if breadcrumb.is_empty() {
        return String::new();
    }

    breadcrumb
        .into_iter()
        .map(|tag| tag.name)
        .collect::<Vec<_>>()
        .join(" > ")
}

pub fn build_tag_filter_options(tags: &[Tag], tag_ids: &[String]) -> Vec<TagFilterOption> {
    let tag_set: HashSet<&str> = tag_ids.iter().map(|id| id.as_str()).collect();
    let mut options: Vec<TagFilterOption> = tags
        .iter()
        .filter(|tag| tag_set.contains(tag.id.as_str()))
        .map(|tag| {
            let label = tag_path_label(tags, &tag.id);
            TagFilterOption {
                id: tag.id.clone(),
                label: if label.is_empty() {
                    tag.name.clone()
                } else {
                    label
                },
                color: tag.color.clone(),
            }
        })
        .collect();

    options.sort_by(|left, right| left.label.to_lowercase().cmp(&right.label.to_lowercase()));
    options
}

pub fn build_container_breadcrumbs(
    current_container_id: &str,
    all_containers: &[Container],
    include_current: bool,
) -> Vec<BreadcrumbCrumb> {
    let mut current_id = Some(current_container_id.to_string());
    let mut chain = Vec::new();
    let mut depth = 0;

    while let Some(ref container_id) = current_id.clone() {
        if depth > 10 {
            break;
        }
        if let Some(container) = all_containers.iter().find(|c| &c.id == container_id) {
            chain.push((
                container.name.clone(),
                format!("/containers/{}", container.id),
            ));
            current_id = container.parent_container_id.clone();
        } else {
            break;
        }
        depth += 1;
    }

    chain.reverse();
    if !include_current {
        chain.pop();
    }
    chain
}

pub fn build_list_ancestor_breadcrumbs(ancestor_lists: &[List]) -> Vec<BreadcrumbCrumb> {
    ancestor_lists
        .iter()
        .map(|list| (list.name.clone(), format!("/lists/{}", list.id)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_shared::{ListFeature, ListType};

    fn sample_tags() -> Vec<Tag> {
        vec![
            Tag {
                id: "root".into(),
                user_id: "u1".into(),
                name: "Praca".into(),
                color: "#111111".into(),
                parent_tag_id: None,
                created_at: "2024-01-01".into(),
            },
            Tag {
                id: "child".into(),
                user_id: "u1".into(),
                name: "Backend".into(),
                color: "#222222".into(),
                parent_tag_id: Some("root".into()),
                created_at: "2024-01-01".into(),
            },
            Tag {
                id: "other".into(),
                user_id: "u1".into(),
                name: "Dom".into(),
                color: "#333333".into(),
                parent_tag_id: None,
                created_at: "2024-01-01".into(),
            },
        ]
    }

    #[test]
    fn tag_path_label_joins_full_breadcrumb() {
        let tags = sample_tags();
        assert_eq!(tag_path_label(&tags, "child"), "Praca > Backend");
    }

    #[test]
    fn build_tag_filter_options_only_keeps_requested_tags_and_sorts_by_label() {
        let tags = sample_tags();
        let options = build_tag_filter_options(
            &tags,
            &[
                "other".to_string(),
                "child".to_string(),
                "missing".to_string(),
            ],
        );

        assert_eq!(
            options,
            vec![
                TagFilterOption {
                    id: "other".into(),
                    label: "Dom".into(),
                    color: "#333333".into(),
                },
                TagFilterOption {
                    id: "child".into(),
                    label: "Praca > Backend".into(),
                    color: "#222222".into(),
                },
            ]
        );
    }

    #[test]
    fn build_container_breadcrumbs_keeps_full_path_when_requested() {
        let containers = vec![
            Container {
                id: "root".into(),
                user_id: "u1".into(),
                name: "Root".into(),
                description: None,
                status: None,
                parent_container_id: None,
                position: 0,
                pinned: false,
                last_opened_at: None,
                created_at: "2024-01-01".into(),
                updated_at: "2024-01-01".into(),
            },
            Container {
                id: "child".into(),
                user_id: "u1".into(),
                name: "Child".into(),
                description: None,
                status: None,
                parent_container_id: Some("root".into()),
                position: 0,
                pinned: false,
                last_opened_at: None,
                created_at: "2024-01-01".into(),
                updated_at: "2024-01-01".into(),
            },
        ];

        let crumbs = build_container_breadcrumbs("child", &containers, true);
        assert_eq!(
            crumbs,
            vec![
                ("Root".into(), "/containers/root".into()),
                ("Child".into(), "/containers/child".into()),
            ]
        );
    }

    #[test]
    fn build_list_ancestor_breadcrumbs_maps_in_order() {
        let ancestors = vec![
            List {
                id: "l1".into(),
                user_id: "u1".into(),
                name: "Main".into(),
                description: None,
                list_type: ListType::Checklist,
                parent_list_id: None,
                position: 0,
                archived: false,
                features: Vec::<ListFeature>::new(),
                container_id: Some("child".into()),
                pinned: false,
                last_opened_at: None,
                created_at: "2024-01-01".into(),
                updated_at: "2024-01-01".into(),
            },
            List {
                id: "l2".into(),
                user_id: "u1".into(),
                name: "Sub".into(),
                description: None,
                list_type: ListType::Checklist,
                parent_list_id: Some("l1".into()),
                position: 1,
                archived: false,
                features: Vec::<ListFeature>::new(),
                container_id: None,
                pinned: false,
                last_opened_at: None,
                created_at: "2024-01-01".into(),
                updated_at: "2024-01-01".into(),
            },
        ];

        let crumbs = build_list_ancestor_breadcrumbs(&ancestors);
        assert_eq!(
            crumbs,
            vec![
                ("Main".into(), "/lists/l1".into()),
                ("Sub".into(), "/lists/l2".into()),
            ]
        );
    }
}
