pub mod detail;

use leptos::prelude::*;

use kartoteka_shared::ChangeTopic;

use crate::components::common::loading::LoadingSpinner;
use crate::components::tags::color_picker::TagColorPicker;
use crate::components::tags::tag_tree::{TagTreeRow, build_tag_tree};
use crate::server_fns::tags::{create_tag, delete_tag, get_all_tags};
use crate::state::invalidation::{mutate, use_invalidation};

#[component]
pub fn TagsPage() -> impl IntoView {
    let invalidation = use_invalidation();

    let tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags]),
        |_| get_all_tags(),
    );

    let (new_name, set_new_name) = signal(String::new());
    let (new_color, set_new_color) = signal("#6366f1".to_string());

    let create_with_parent = move |name: String, parent_tag_id: Option<String>| {
        let color = new_color.get_untracked();
        mutate(
            create_tag(name, None, Some(color), parent_tag_id),
            &[ChangeTopic::Tags],
        );
    };

    let on_create_root = Callback::new(move |_: ()| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        create_with_parent(name, None);
        set_new_name.set(String::new());
    });

    let on_create_child = Callback::new(move |(parent_id, name): (String, String)| {
        create_with_parent(name, Some(parent_id));
    });

    let on_delete = Callback::new(move |id: String| {
        mutate(
            delete_tag(id),
            &[ChangeTopic::Tags, ChangeTopic::Items, ChangeTopic::Lists],
        );
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Tagi"</h2>

            // Create-root form
            <div class="flex gap-2 mb-6">
                <TagColorPicker
                    value=Signal::derive(move || new_color.get())
                    on_change=Callback::new(move |c: String| set_new_color.set(c))
                />
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder="Nowy tag..."
                    data-testid="new-tag-input"
                    prop:value=move || new_name.get()
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            on_create_root.run(());
                        }
                    }
                />
                <button
                    type="button"
                    class="btn btn-primary"
                    data-testid="create-tag-btn"
                    on:click=move |_| on_create_root.run(())
                >
                    "Dodaj"
                </button>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || tags_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(tags) => {
                        if tags.is_empty() {
                            return view! {
                                <div class="text-center text-base-content/50 py-8" data-testid="tags-empty-state">
                                    "Brak tagów. Dodaj pierwszy powyżej."
                                </div>
                            }.into_any();
                        }
                        let tree = build_tag_tree(&tags);
                        view! {
                            <div data-testid="tag-tree">
                                {tree.into_iter().map(|node| {
                                    view! {
                                        <TagTreeRow
                                            node=node
                                            depth=0
                                            on_create_child=on_create_child
                                            on_delete=on_delete
                                        />
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
