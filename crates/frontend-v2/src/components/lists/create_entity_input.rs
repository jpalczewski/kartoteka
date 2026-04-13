use kartoteka_shared::types::{CreateContainerRequest, CreateListRequest};
use leptos::prelude::*;

use super::add_input::AddInput;

#[derive(Clone, PartialEq)]
enum EntityMode {
    List,
    Folder,
    Project,
}

#[component]
pub fn CreateEntityInput(
    #[prop(optional)] parent_container_id: Option<String>,
    #[prop(default = true)] show_container_options: bool,
    on_create_list: Callback<CreateListRequest>,
    on_create_container: Callback<CreateContainerRequest>,
) -> impl IntoView {
    let (mode, set_mode) = signal(EntityMode::List);
    let (list_type, set_list_type) = signal("checklist".to_string());

    let parent_id = parent_container_id.clone();

    let on_submit = Callback::new(move |name: String| {
        let m = mode.get();
        match m {
            EntityMode::List => {
                on_create_list.run(CreateListRequest {
                    name,
                    list_type: Some(list_type.get()),
                    icon: None,
                    description: None,
                    container_id: parent_id.clone(),
                    parent_list_id: None,
                    features: vec![],
                });
            }
            EntityMode::Folder => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    icon: None,
                    description: None,
                    status: None,
                    parent_container_id: parent_id.clone(),
                });
            }
            EntityMode::Project => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    icon: None,
                    description: None,
                    status: Some("active".to_string()),
                    parent_container_id: parent_id.clone(),
                });
            }
        }
    });

    let list_type_options: &[&str] = &["checklist", "zakupy", "pakowanie", "terminarz"];

    view! {
        <div class="mb-4">
            <div class="tabs tabs-boxed mb-2">
                <a
                    class=move || if mode.get() == EntityMode::List { "tab tab-active" } else { "tab" }
                    on:click=move |_| set_mode.set(EntityMode::List)
                >
                    "Lista"
                </a>
                {show_container_options.then(|| view! {
                    <div class="contents">
                        <a
                            class=move || if mode.get() == EntityMode::Folder { "tab tab-active" } else { "tab" }
                            on:click=move |_| set_mode.set(EntityMode::Folder)
                        >
                            "Folder"
                        </a>
                        <a
                            class=move || if mode.get() == EntityMode::Project { "tab tab-active" } else { "tab" }
                            on:click=move |_| set_mode.set(EntityMode::Project)
                        >
                            "Projekt"
                        </a>
                    </div>
                })}
            </div>

            {move || (mode.get() == EntityMode::List).then(|| view! {
                <div class="flex gap-2 mb-2 flex-wrap">
                    {list_type_options.iter().map(|lt| {
                        let lt_cmp = lt.to_string();
                        let lt_set = lt.to_string();
                        view! {
                            <label class="flex items-center gap-1 cursor-pointer">
                                <input
                                    type="radio"
                                    name="list-type"
                                    class="radio radio-sm"
                                    prop:checked=move || list_type.get() == lt_cmp
                                    on:change=move |_| set_list_type.set(lt_set.clone())
                                />
                                {*lt}
                            </label>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            })}

            <AddInput
                placeholder=Signal::derive(move || {
                    match mode.get() {
                        EntityMode::List => "Nazwa listy...".to_string(),
                        EntityMode::Folder => "Nazwa folderu...".to_string(),
                        EntityMode::Project => "Nazwa projektu...".to_string(),
                    }
                })
                button_label=Signal::derive(move || "Utwórz".to_string())
                on_submit=on_submit
            />
        </div>
    }
}
