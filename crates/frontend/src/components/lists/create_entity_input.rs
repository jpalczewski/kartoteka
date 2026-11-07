use kartoteka_shared::types::{CreateContainerRequest, CreateListRequest};
use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use super::add_input::AddInput;

#[derive(Clone, Copy, PartialEq)]
enum EntityMode {
    List,
    Folder,
    Project,
}

/// Tabs in display order with their `data-testid`.
const MODES: &[(EntityMode, &str)] = &[
    (EntityMode::List, "create-entity-tab-list"),
    (EntityMode::Folder, "create-entity-tab-folder"),
    (EntityMode::Project, "create-entity-tab-project"),
];

/// Moves keyboard focus to the tab with the given test id (client only).
#[cfg(target_arch = "wasm32")]
fn focus_tab(testid: &str) {
    use web_sys::wasm_bindgen::JsCast;
    let selector = format!("[data-testid=\"{testid}\"]");
    if let Some(el) = document().query_selector(&selector).ok().flatten()
        && let Ok(el) = el.dyn_into::<web_sys::HtmlElement>()
    {
        let _ = el.focus();
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn focus_tab(_testid: &str) {}

#[component]
pub fn CreateEntityInput(
    #[prop(optional)] parent_container_id: Option<String>,
    #[prop(default = true)] show_container_options: bool,
    on_create_list: Callback<CreateListRequest>,
    on_create_container: Callback<CreateContainerRequest>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
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
                    color: None,
                    inherit_from_parent: true,
                });
            }
            EntityMode::Folder => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    icon: None,
                    description: None,
                    status: None,
                    parent_container_id: parent_id.clone(),
                    color: None,
                });
            }
            EntityMode::Project => {
                on_create_container.run(CreateContainerRequest {
                    name,
                    icon: None,
                    description: None,
                    status: Some("active".to_string()),
                    parent_container_id: parent_id.clone(),
                    color: None,
                });
            }
        }
    });

    let list_type_options: &[&str] = &["checklist", "shopping", "schedule", "log", "notes"];

    view! {
        <div class="mb-4">
            <div
                role="tablist"
                class="tabs tabs-box mb-2"
                aria-label=move_tr!("lists-create-kind-aria")
                data-testid="create-entity-tabs"
                on:keydown=move |ev: KeyboardEvent| {
                    // Arrow keys move between tabs (roving tabindex), like a native tablist.
                    let step: isize = match ev.key().as_str() {
                        "ArrowRight" => 1,
                        "ArrowLeft" => -1,
                        _ => return,
                    };
                    let available = if show_container_options { MODES } else { &MODES[..1] };
                    let current = available.iter().position(|(m, _)| *m == mode.get_untracked()).unwrap_or(0);
                    let next = (current as isize + step).rem_euclid(available.len() as isize) as usize;
                    let (next_mode, testid) = available[next];
                    set_mode.set(next_mode);
                    ev.prevent_default();
                    focus_tab(testid);
                }
            >
                {MODES.iter().filter(|(m, _)| show_container_options || *m == EntityMode::List).map(|(kind, testid)| {
                    let kind = *kind;
                    view! {
                        <button
                            type="button"
                            role="tab"
                            class=move || if mode.get() == kind { "tab tab-active" } else { "tab" }
                            aria-selected=move || (mode.get() == kind).to_string()
                            tabindex=move || if mode.get() == kind { "0" } else { "-1" }
                            data-testid=*testid
                            on:click=move |_| set_mode.set(kind)
                        >
                            {match kind {
                                EntityMode::List => move_tr!("lists-mode-list"),
                                EntityMode::Folder => move_tr!("lists-mode-folder"),
                                EntityMode::Project => move_tr!("lists-mode-project"),
                            }}
                        </button>
                    }
                }).collect_view()}
            </div>

            {move || (mode.get() == EntityMode::List).then(|| view! {
                <div class="flex gap-2 mb-2 flex-wrap">
                    {list_type_options.iter().map(|lt| {
                        let lt_cmp = lt.to_string();
                        let lt_set = lt.to_string();
                        let label = i18n.tr(&format!("lists-type-{lt}"));
                        view! {
                            <label class="flex items-center gap-1 cursor-pointer">
                                <input
                                    type="radio"
                                    name="list-type"
                                    class="radio radio-sm"
                                    data-testid=format!("list-type-{lt}")
                                    prop:checked=move || list_type.get() == lt_cmp
                                    on:change=move |_| set_list_type.set(lt_set.clone())
                                />
                                {label}
                            </label>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            })}

            <AddInput
                placeholder=Signal::derive(move || {
                    match mode.get() {
                        EntityMode::List => i18n.tr("lists-new-list-placeholder"),
                        EntityMode::Folder => i18n.tr("lists-new-folder-placeholder"),
                        EntityMode::Project => i18n.tr("lists-new-project-placeholder"),
                    }
                })
                button_label=Signal::derive(move || i18n.tr("common-create"))
                on_submit=on_submit
                testid="create-entity-input"
            />
        </div>
    }
}
