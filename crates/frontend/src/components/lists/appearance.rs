//! Page-level host for the icon + colour dialog of lists and containers.
//!
//! Cards are rebuilt whenever the page data refetches (every mutation and live
//! update), which would destroy a dialog living inside one and lose the draft. So a
//! page provides one [`AppearanceHost`] with [`provide_appearance_host`], renders a
//! single [`AppearanceModal`] outside its refetching sections, and cards open it
//! through [`appearance_menu_item`] — the same split as `DeleteEntityModal`.

use kartoteka_shared::types::{Container, List};
use kartoteka_shared::{ChangeTopic, default_container_icon, default_list_icon};
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::components::ui::{ActionMenuItem, AppearanceDialog};
use crate::server_fns::containers::set_container_appearance;
use crate::server_fns::lists::set_list_appearance;
use crate::state::invalidation::mutate;

/// Argument that clears a field in `set_*_appearance`: the server treats a blank string as
/// "clear" and `None` as "keep". It has to be non-empty because the urlencoded server-fn
/// encoding turns `Some("")` into `None` on the wire.
const CLEAR: &str = " ";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppearanceKind {
    List,
    Container,
}

/// The entity whose appearance is being edited, with its stored values.
#[derive(Clone, PartialEq, Debug)]
pub struct AppearanceTarget {
    pub kind: AppearanceKind,
    pub id: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    /// Lucide name used when the icon is cleared.
    pub default_icon: &'static str,
}

impl AppearanceTarget {
    pub fn list(list: &List) -> Self {
        Self {
            kind: AppearanceKind::List,
            id: list.id.clone(),
            icon: list.icon.clone(),
            color: list.color.clone(),
            default_icon: default_list_icon(&list.list_type),
        }
    }

    pub fn container(container: &Container) -> Self {
        Self {
            kind: AppearanceKind::Container,
            id: container.id.clone(),
            icon: container.icon.clone(),
            color: container.color.clone(),
            default_icon: default_container_icon(container.status.is_some()),
        }
    }
}

/// Context handle through which cards ask the page to open the dialog.
#[derive(Clone, Copy)]
pub struct AppearanceHost(RwSignal<Option<AppearanceTarget>>);

/// Creates the host and provides it to descendants; render [`AppearanceModal`] once
/// on the same page, outside any section that refetches.
pub fn provide_appearance_host() -> AppearanceHost {
    let host = AppearanceHost(RwSignal::new(None));
    provide_context(host);
    host
}

/// "Appearance…" entry for a card's `ActionMenu`, or `None` when the page has no host.
pub fn appearance_menu_item(
    target: AppearanceTarget,
    testid: &'static str,
) -> Option<ActionMenuItem> {
    let host = use_context::<AppearanceHost>()?;
    Some(
        ActionMenuItem::new(
            "palette",
            move_tr!("appearance-action"),
            Callback::new(move |_: ()| host.0.set(Some(target.clone()))),
        )
        .testid(testid),
    )
}

/// The one appearance dialog of a page; saves through the matching server fn.
#[component]
pub fn AppearanceModal() -> impl IntoView {
    let host = expect_context::<AppearanceHost>();

    // Re-created only when another entity is opened (or the dialog closes), not on
    // data refetches, so the draft survives live updates.
    view! {
        {move || host.0.get().map(|target| {
            let open = RwSignal::new(true);
            Effect::new(move |_| {
                if !open.get() {
                    host.0.set(None);
                }
            });
            let AppearanceTarget { kind, id, icon, color, default_icon } = target;
            let on_save = Callback::new(move |(icon, color): (Option<String>, Option<String>)| {
                let icon = Some(icon.unwrap_or_else(|| CLEAR.to_string()));
                let color = Some(color.unwrap_or_else(|| CLEAR.to_string()));
                let id = id.clone();
                match kind {
                    AppearanceKind::List => mutate(
                        set_list_appearance(id, icon, color),
                        &[ChangeTopic::Lists],
                    ),
                    AppearanceKind::Container => mutate(
                        set_container_appearance(id, icon, color),
                        &[ChangeTopic::Containers],
                    ),
                }
            });
            view! {
                <AppearanceDialog
                    open=open
                    icon=icon
                    color=color
                    default_icon=default_icon
                    on_save=on_save
                />
            }
        })}
    }
}
