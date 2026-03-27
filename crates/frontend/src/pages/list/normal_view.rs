use leptos::prelude::*;

use crate::components::items::item_row::ItemRow;
use kartoteka_shared::{Item, ItemTagLink, List, Tag};

pub struct NormalViewProps {
    pub items: Vec<Item>,
    pub tags: Vec<Tag>,
    pub item_tag_links: RwSignal<Vec<ItemTagLink>>,
    pub sublists: Vec<List>,
    pub on_toggle: Callback<String>,
    pub on_delete: Callback<String>,
    pub on_tag_toggle: Callback<(String, String)>,
    pub on_description_save: Callback<(String, String)>,
    pub on_quantity_change: Callback<(String, i32)>,
    pub has_quantity: bool,
    pub on_move: Callback<(String, String)>,
    pub on_date_save: Callback<(String, String, String, Option<String>)>,
    pub deadlines_config: serde_json::Value,
}

#[allow(clippy::too_many_arguments)]
pub fn render_normal_view(p: NormalViewProps) -> impl IntoView {
    let NormalViewProps {
        items,
        tags,
        item_tag_links,
        sublists,
        on_toggle,
        on_delete,
        on_tag_toggle,
        on_description_save,
        on_quantity_change,
        has_quantity,
        on_move,
        on_date_save,
        deadlines_config,
    } = p;
    let move_targets: Vec<(String, String)> = sublists
        .iter()
        .map(|s| (s.id.clone(), s.name.clone()))
        .collect();

    view! {
        <div>
            {items.iter().map(|item| {
                let item_id = item.id.clone();
                let item_tags: Vec<String> = item_tag_links.read().iter()
                    .filter(|l| l.item_id == item.id)
                    .map(|l| l.tag_id.clone())
                    .collect();
                let tags_clone = tags.clone();
                let item_tag_toggle = Callback::new(move |tag_id: String| {
                    on_tag_toggle.run((item_id.clone(), tag_id));
                });
                let mt = move_targets.clone();
                let dc = deadlines_config.clone();
                view! {
                    <ItemRow
                        item=item.clone()
                        on_toggle=on_toggle
                        on_delete=on_delete
                        all_tags=tags_clone
                        item_tag_ids=item_tags
                        on_tag_toggle=item_tag_toggle
                        on_description_save=on_description_save
                        has_quantity=has_quantity
                        on_quantity_change=on_quantity_change
                        move_targets=mt
                        on_move=on_move
                        on_date_save=on_date_save
                        deadlines_config=dc
                    />
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
