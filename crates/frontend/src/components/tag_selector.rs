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
        <div class="relative">
            <button class="btn btn-ghost btn-xs btn-square" on:click=move |_| set_open.update(|v| *v = !*v)>
                "＋"
            </button>
            <div
                class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                style:display=move || if open.get() { "block" } else { "none" }
            >
                {all_tags.into_iter().map(|tag| {
                    let is_selected = selected_tag_ids.contains(&tag.id);
                    let color = tag.color.clone();
                    let name = tag.name.clone();
                    let tid = tag.id.clone();
                    view! {
                        <label
                            class="flex items-center gap-2 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300"
                            style=format!("border-left: 3px solid {color};")
                        >
                            <input
                                type="checkbox"
                                class="checkbox checkbox-secondary checkbox-xs"
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
