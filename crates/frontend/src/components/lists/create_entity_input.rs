use kartoteka_shared::{
    ContainerStatus, CreateContainerRequest, CreateListRequest, FEATURE_DEADLINES,
    FEATURE_QUANTITY, ListFeature, ListType,
};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::items::add_input::AddInput;
use crate::components::list_card::list_type_icon;

#[derive(Clone, PartialEq)]
pub enum EntityMode {
    List,
    Folder,
    Project,
}

#[component]
pub fn CreateEntityInput(
    #[prop(optional)] parent_container_id: Option<String>,
    /// Whether to show folder/project options (false for projects which can't nest containers)
    #[prop(default = true)]
    show_container_options: bool,
    on_create_list: Callback<CreateListRequest>,
    on_create_container: Callback<CreateContainerRequest>,
) -> impl IntoView {
    let (mode, set_mode) = signal(EntityMode::List);
    let (new_list_type, set_new_list_type) = signal(ListType::Custom);
    let (feat_quantity, set_feat_quantity) = signal(false);
    let (feat_deadlines, set_feat_deadlines) = signal(false);

    let parent_id = parent_container_id.clone();

    let on_submit = Callback::new(move |name: String| {
        let m = mode.get();
        match m {
            EntityMode::List => {
                let fq = feat_quantity.get();
                let fd = feat_deadlines.get();
                let lt = new_list_type.get();
                let mut features = Vec::new();
                if fq {
                    features.push(ListFeature {
                        name: FEATURE_QUANTITY.into(),
                        config: serde_json::json!({"unit_default": "szt"}),
                    });
                }
                if fd {
                    features.push(ListFeature {
                        name: FEATURE_DEADLINES.into(),
                        config: serde_json::json!({"has_start_date": false, "has_deadline": true, "has_hard_deadline": false}),
                    });
                }
                on_create_list.run(CreateListRequest {
                    name,
                    list_type: lt,
                    features: Some(features),
                    parent_list_id: None,
                    container_id: None,
                });
            }
            EntityMode::Folder => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    status: None,
                    parent_container_id: parent_id.clone(),
                });
            }
            EntityMode::Project => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    status: Some(ContainerStatus::Active),
                    parent_container_id: parent_id.clone(),
                });
            }
        }
    });

    view! {
        <div class="mb-4">
            // Mode tabs
            <div class="tabs tabs-boxed mb-2 w-fit">
                <button
                    type="button"
                    class=move || if mode.get() == EntityMode::List { "tab tab-active" } else { "tab" }
                    on:click=move |_| set_mode.set(EntityMode::List)
                >
                    "📝 " {move_tr!("lists-mode-list")}
                </button>
                {if show_container_options {
                    view! {
                        <>
                            <button
                                type="button"
                                class=move || if mode.get() == EntityMode::Folder { "tab tab-active" } else { "tab" }
                                on:click=move |_| set_mode.set(EntityMode::Folder)
                            >
                                "📁 " {move_tr!("lists-mode-folder")}
                            </button>
                            <button
                                type="button"
                                class=move || if mode.get() == EntityMode::Project { "tab tab-active" } else { "tab" }
                                on:click=move |_| set_mode.set(EntityMode::Project)
                            >
                                "🚀 " {move_tr!("lists-mode-project")}
                            </button>
                        </>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>

            // List options (only when mode=List)
            {move || if mode.get() == EntityMode::List {
                view! {
                    <div>
                        <div class="flex flex-wrap gap-2 mb-2">
                            {[ListType::Checklist, ListType::Zakupy, ListType::Pakowanie, ListType::Terminarz, ListType::Custom]
                                .into_iter()
                                .map(|lt| {
                                    let lt_class = lt.clone();
                                    let lt_click = lt.clone();
                                    let icon = list_type_icon(&lt);
                                    let type_label = match &lt {
                                        ListType::Checklist => move_tr!("lists-type-checklist"),
                                        ListType::Zakupy => move_tr!("lists-type-shopping"),
                                        ListType::Pakowanie => move_tr!("lists-type-packing"),
                                        ListType::Terminarz => move_tr!("lists-type-schedule"),
                                        ListType::Custom => move_tr!("lists-type-custom"),
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=move || if new_list_type.get() == lt_class { "btn btn-sm btn-primary" } else { "btn btn-sm btn-outline" }
                                            on:click=move |_| {
                                                set_new_list_type.set(lt_click.clone());
                                                let defaults = lt_click.default_features();
                                                set_feat_quantity.set(defaults.iter().any(|f| f.name == FEATURE_QUANTITY));
                                                set_feat_deadlines.set(defaults.iter().any(|f| f.name == FEATURE_DEADLINES));
                                            }
                                        >
                                            {icon} " " {type_label}
                                        </button>
                                    }
                                })
                                .collect::<Vec<_>>()}
                        </div>
                        <div class="flex items-center gap-4 mb-2">
                            <label class="label cursor-pointer gap-2">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-sm"
                                    prop:checked=feat_quantity
                                    on:change=move |ev| set_feat_quantity.set(event_target_checked(&ev))
                                />
                                <span class="label-text">{move_tr!("lists-feature-quantities")}</span>
                            </label>
                            <label class="label cursor-pointer gap-2">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-sm"
                                    prop:checked=feat_deadlines
                                    on:change=move |ev| set_feat_deadlines.set(event_target_checked(&ev))
                                />
                                <span class="label-text">{move_tr!("lists-feature-deadlines")}</span>
                            </label>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Input
            <div class="flex gap-2">
                <AddInput
                    placeholder=Signal::derive(move || match mode.get() {
                        EntityMode::List => move_tr!("lists-new-list-placeholder").get(),
                        EntityMode::Folder => move_tr!("lists-new-folder-placeholder").get(),
                        EntityMode::Project => move_tr!("lists-new-project-placeholder").get(),
                    })
                    button_label=move_tr!("common-add")
                    on_submit=on_submit
                />
            </div>
        </div>
    }
}
