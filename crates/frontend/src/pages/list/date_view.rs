use leptos::prelude::*;

use crate::components::common::date_utils::{get_today, is_overdue, is_upcoming, sort_by_deadline};
use crate::components::items::date_item_row::DateItemRow;
use kartoteka_shared::{Item, ItemTagLink, Tag};

pub fn render_date_view(
    all: Vec<Item>,
    tags: Vec<Tag>,
    links: Vec<ItemTagLink>,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    on_tag_toggle: Callback<(String, String)>,
    on_date_save: Callback<(String, String, String, Option<String>)>,
) -> impl IntoView {
    let today = get_today();

    let mut overdue: Vec<Item> = all
        .iter()
        .filter(|i| is_overdue(i, &today))
        .cloned()
        .collect();
    sort_by_deadline(&mut overdue);

    let mut upcoming: Vec<Item> = all
        .iter()
        .filter(|i| is_upcoming(i, &today))
        .cloned()
        .collect();
    sort_by_deadline(&mut upcoming);

    let mut done: Vec<Item> = all.iter().filter(|i| i.completed).cloned().collect();
    sort_by_deadline(&mut done);

    let render_section = |label: &str,
                          css: &str,
                          items: Vec<Item>,
                          tags: Vec<Tag>,
                          links: Vec<ItemTagLink>| {
        let label = label.to_string();
        let css = css.to_string();
        if items.is_empty() {
            ().into_any()
        } else {
            view! {
                <div class="mb-4">
                    <h3 class=format!("text-sm font-semibold uppercase tracking-wide mb-2 {css}")>{label}</h3>
                    {items.into_iter().map(|item| {
                        let item_id = item.id.clone();
                        let item_tags: Vec<String> = links.iter()
                            .filter(|l| l.item_id == item.id)
                            .map(|l| l.tag_id.clone())
                            .collect();
                        let tags_clone = tags.clone();
                        let item_tag_toggle = Callback::new(move |tag_id: String| {
                            on_tag_toggle.run((item_id.clone(), tag_id));
                        });
                        view! {
                            <DateItemRow
                                item=item
                                on_toggle=on_toggle
                                on_delete=on_delete
                                all_tags=tags_clone
                                item_tag_ids=item_tags
                                on_tag_toggle=item_tag_toggle
                                on_date_save=on_date_save
                            />
                        }
                    }).collect::<Vec<_>>()}
                </div>
            }.into_any()
        }
    };

    view! {
        <div>
            {render_section("Zaleg\u{0142}e", "text-error", overdue, tags.clone(), links.clone())}
            {render_section("Nadchodz\u{0105}ce", "text-warning", upcoming, tags.clone(), links.clone())}
            {render_section("Zrobione", "text-base-content/40", done, tags, links)}
        </div>
    }
}
