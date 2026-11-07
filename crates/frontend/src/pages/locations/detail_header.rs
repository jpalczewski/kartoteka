use leptos::prelude::*;
use leptos_fluent::I18n;
use leptos_router::hooks::use_navigate;

use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::inline_edit::InlineEdit;
use crate::server_fns::locations::{delete_location_sf, update_location_sf};
use crate::state::invalidation::{mutate, mutate_with};
use kartoteka_shared::{ChangeTopic, UpdateLocationRequest};

use super::InlineAlias;

/// Header shared by the city and address detail pages.
///
/// Owns the rename, alias and delete actions so the pages only supply data and the
/// extras they add (the city's region/country line) as `children`.
///
/// Props (documented here because rustc forbids doc comments on parameters):
/// - `id`: location being shown; renames and the delete act on it.
/// - `name`: current name, shown as the title and used as the rename draft.
/// - `alias`: current alias, rendered by [`InlineAlias`].
/// - `after_delete_href`: where to navigate once the delete succeeds.
/// - `children`: extra content between the title and the alias.
#[component]
pub(super) fn LocationDetailHeader(
    id: String,
    name: String,
    alias: Option<String>,
    after_delete_href: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let navigate = use_navigate();
    let id = StoredValue::new(id);
    let name = StoredValue::new(name);
    let after_delete_href = StoredValue::new(after_delete_href);

    let (editing, set_editing) = signal(false);
    let show_delete = RwSignal::new(false);

    let on_save = Callback::new(move |new_name: String| {
        set_editing.set(false);
        mutate(
            update_location_sf(id.get_value(), UpdateLocationRequest::rename(new_name)),
            &[ChangeTopic::Locations],
        );
    });

    let delete_label = i18n.tr("locations-delete-title");
    let on_delete = Callback::new(move |_: ()| {
        show_delete.set(false);
        let id = id.get_value();
        let href = after_delete_href.get_value();
        let navigate = navigate.clone();
        mutate_with(
            delete_location_sf(id),
            &[ChangeTopic::Locations],
            move |_| navigate(&href, Default::default()),
        );
    });

    view! {
        <div class="flex items-start justify-between mb-6">
            <div>
                {move || {
                    if editing.get() {
                        view! {
                            <InlineEdit
                                value=name.get_value()
                                on_save=on_save
                                on_cancel=Callback::new(move |_: ()| set_editing.set(false))
                            />
                        }
                        .into_any()
                    } else {
                        view! {
                            <div class="flex items-center gap-2">
                                <h2 class="text-2xl font-bold">{name.get_value()}</h2>
                                <button
                                    class="btn btn-xs btn-ghost"
                                    on:click=move |_| set_editing.set(true)
                                >
                                    "✎"
                                </button>
                            </div>
                        }
                        .into_any()
                    }
                }}

                {children.map(|children| children())}

                <div class="mt-2">
                    <InlineAlias id=id.get_value() initial=alias />
                </div>
            </div>
            <button
                class="btn btn-sm btn-ghost text-error"
                on:click=move |_| show_delete.set(true)
            >
                {delete_label}
            </button>
        </div>

        <ConfirmModal
            open=Signal::from(show_delete)
            title=i18n.tr("locations-delete-title")
            message=i18n.tr("locations-delete-confirm")
            confirm_label=i18n.tr("common-delete")
            variant=ConfirmVariant::Danger
            on_confirm=on_delete
            on_close=Callback::new(move |_: ()| show_delete.set(false))
        />
    }
}
