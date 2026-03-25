use crate::api;
use crate::components::add_input::AddInput;
use crate::components::tag_badge::TagBadge;
use crate::components::tag_tree::{build_tag_tree, TagNode};
use kartoteka_shared::{CreateTagRequest, Tag};
use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn TagsPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let tags = RwSignal::new(Vec::<Tag>::new());
    let (loading, set_loading) = signal(true);
    let (new_color, set_new_color) = signal("#e94560".to_string());
    // Which parent tag is currently in "add child" mode (None = top-level add only)
    let adding_child_to = RwSignal::new(Option::<String>::None);

    // Initial fetch
    leptos::task::spawn_local(async move {
        if let Ok(fetched) = api::fetch_tags().await {
            tags.set(fetched);
        }
        set_loading.set(false);
    });

    let do_create = Callback::new(move |(name, parent_id): (String, Option<String>)| {
        let color = new_color.get_untracked();
        leptos::task::spawn_local(async move {
            let req = CreateTagRequest {
                name,
                color,
                parent_tag_id: parent_id,
            };
            if let Ok(tag) = api::create_tag(&req).await {
                tags.update(|t| t.push(tag));
            }
            adding_child_to.set(None);
        });
    });

    let on_create_root = Callback::new(move |name: String| {
        do_create.run((name, None));
    });

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
                    type="color"
                    aria-label="Kolor tagu"
                    class="w-8 h-8 rounded cursor-pointer border-0 p-0"
                    prop:value=move || new_color.get()
                    on:input=move |ev| set_new_color.set(event_target_value(&ev))
                />
                <AddInput placeholder="Nowy tag..." button_label="Dodaj" on_submit=on_create_root />
            </div>

            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }
                let all_tags = tags.get();
                if all_tags.is_empty() {
                    return view! { <p class="text-center text-base-content/50 py-12">"Brak tagów. Dodaj pierwszy!"</p> }.into_any();
                }

                let tree = build_tag_tree(&all_tags);
                view! {
                    <div>
                        {tree.into_iter().map(|node| {
                            view! {
                                <TagTreeRow
                                    node=node
                                    depth=0
                                    on_delete=on_delete
                                    adding_child_to=adding_child_to
                                    do_create=do_create
                                />
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}

#[component]
fn TagTreeRow(
    node: TagNode,
    depth: usize,
    on_delete: Callback<String>,
    adding_child_to: RwSignal<Option<String>>,
    do_create: Callback<(String, Option<String>)>,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let tid = tag.id.clone();
    let tid_link = tag.id.clone();
    let tid_add = tag.id.clone();
    let tid_delete = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 1.0);

    view! {
        <div>
            <div class="flex items-center gap-2 py-1" style=padding.clone()>
                <A href=format!("/tags/{tid_link}") attr:class="no-underline">
                    <TagBadge tag=tag.clone() />
                </A>
                <button
                    class="btn btn-ghost btn-xs btn-square"
                    title="Dodaj podtag"
                    on:click=move |_| {
                        adding_child_to.set(Some(tid_add.clone()));
                    }
                >"+"</button>
                <button
                    class="btn btn-error btn-xs btn-square"
                    on:click=move |_| on_delete.run(tid_delete.clone())
                >"✕"</button>
            </div>

            // Inline add-child form
            {move || {
                let current = adding_child_to.get();
                if current.as_deref() == Some(&tid) {
                    let tid_for_create = tid.clone();
                    let on_submit_child = Callback::new(move |name: String| {
                        do_create.run((name, Some(tid_for_create.clone())));
                    });
                    let child_padding = format!("padding-left: {}rem;", (depth + 1) as f64 * 1.0);
                    view! {
                        <div class="flex gap-2 items-center py-1" style=child_padding>
                            <AddInput placeholder="Nazwa podtagu..." button_label="Dodaj" on_submit=on_submit_child />
                            <button class="btn btn-ghost btn-xs" on:click=move |_| adding_child_to.set(None)>"✕"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Children
            {children.into_iter().map(|child| {
                view! {
                    <TagTreeRow
                        node=child
                        depth=depth + 1
                        on_delete=on_delete
                        adding_child_to=adding_child_to
                        do_create=do_create
                    />
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}
