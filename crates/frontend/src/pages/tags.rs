use kartoteka_shared::*;
use leptos::prelude::*;

use crate::api;
use crate::components::tag_badge::TagBadge;

fn category_label(cat: &TagCategory) -> &'static str {
    match cat {
        TagCategory::Context => "Kontekst",
        TagCategory::Priority => "Priorytet",
        TagCategory::Custom => "Własne",
    }
}

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_name, set_new_name) = signal(String::new());
    let (new_color, set_new_color) = signal("#e94560".to_string());
    let (new_category, set_new_category) = signal("custom".to_string());

    // Initial fetch
    leptos::task::spawn_local(async move {
        if let Ok(fetched) = api::fetch_tags().await {
            tags.set(fetched);
        }
        set_loading.set(false);
    });

    let on_create = move |_| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        let color = new_color.get();
        let category: TagCategory = match new_category.get().as_str() {
            "context" => TagCategory::Context,
            "priority" => TagCategory::Priority,
            _ => TagCategory::Custom,
        };
        set_new_name.set(String::new());
        leptos::task::spawn_local(async move {
            let req = CreateTagRequest {
                name,
                color,
                category,
                parent_tag_id: None,
            };
            if let Ok(tag) = api::create_tag(&req).await {
                tags.update(|t| t.push(tag));
            }
        });
    };

    let on_delete = Callback::new(move |tag_id: String| {
        tags.update(|t| t.retain(|tag| tag.id != tag_id));
        leptos::task::spawn_local(async move {
            let _ = api::delete_tag(&tag_id).await;
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-6">"Tagi"</h2>

            <div class="flex gap-2 items-center mb-4">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder="Nazwa tagu..."
                    prop:value=move || new_name.get()
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                />
                <input
                    type="color"
                    class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                    prop:value=move || new_color.get()
                    on:input=move |ev| set_new_color.set(event_target_value(&ev))
                />
                <select class="select select-bordered" on:change=move |ev| set_new_category.set(event_target_value(&ev))>
                    <option value="custom">"Własne"</option>
                    <option value="context">"Kontekst"</option>
                    <option value="priority">"Priorytet"</option>
                </select>
                <button class="btn btn-primary btn-sm" on:click=on_create>"Dodaj tag"</button>
            </div>

            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                let all_tags = tags.get();
                if all_tags.is_empty() {
                    return view! { <p class="text-center text-base-content/50 py-12">"Brak tagów. Dodaj pierwszy!"</p> }.into_any();
                }

                let categories = [TagCategory::Context, TagCategory::Priority, TagCategory::Custom];
                view! {
                    <div>
                        {categories.into_iter().map(|cat| {
                            let cat_tags: Vec<Tag> = all_tags.iter()
                                .filter(|t| t.category == cat)
                                .cloned()
                                .collect();
                            if cat_tags.is_empty() {
                                return view! {}.into_any();
                            }
                            let label = category_label(&cat);
                            let del_cb = on_delete.clone();
                            view! {
                                <div class="tag-group mb-4">
                                    <h4 class="text-xs text-base-content/50 uppercase tracking-wider mb-2">{label}</h4>
                                    {cat_tags.into_iter().map(|tag| {
                                        let tid = tag.id.clone();
                                        let cb = del_cb.clone();
                                        view! {
                                            <div class="flex items-center gap-2 py-1">
                                                <TagBadge tag=tag.clone() />
                                                <button class="btn btn-error btn-xs btn-square" on:click=move |_| cb.run(tid.clone())>"✕"</button>
                                            </div>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            }.into_any()
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}
