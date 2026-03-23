use kartoteka_shared::Tag;
use leptos::prelude::*;

#[component]
pub fn TagSelector(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let (open, set_open) = signal(false);

    view! {
        <div class="tag-selector">
            <button class="btn btn-sm" on:click=move |_| set_open.update(|v| *v = !*v)>
                "＋"
            </button>
            <div class="tag-selector-dropdown" style:display=move || if open.get() { "block" } else { "none" }>
                {all_tags.into_iter().map(|tag| {
                    let is_selected = selected_tag_ids.contains(&tag.id);
                    let color = tag.color.clone();
                    let name = tag.name.clone();
                    let tid = tag.id.clone();
                    view! {
                        <label class="tag-option" style=format!("border-left: 3px solid {color};")>
                            <input
                                type="checkbox"
                                checked=is_selected
                                on:change=move |_| on_toggle.run(tid.clone())
                            />
                            {name}
                        </label>
                    }
                }).collect::<Vec<_>>()}
            </div>
        </div>
    }
}
