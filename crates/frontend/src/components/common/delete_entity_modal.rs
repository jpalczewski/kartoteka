//! Shared delete confirmation flow for lists and containers.
//!
//! Deleting a list/container also deletes its subtree, but only when the whole tree is
//! empty: items anywhere in the tree block the deletion, and an empty tree that still
//! has descendants requires an explicit confirmation (`force`). Every delete site in the
//! UI (`home`, list page, container page) goes through [`DeleteEntityModal`] so the
//! wording, the blocking rule and the `force` flag are decided in exactly one place.

use std::collections::HashMap;

use kartoteka_shared::DeletePreview;
use leptos::prelude::*;
use leptos_fluent::I18n;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::server_fns::containers::{delete_container, delete_container_preview};
use crate::server_fns::lists::{delete_list, delete_list_preview};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeleteEntityKind {
    List,
    Container,
}

/// A delete the user asked for; it is turned into a preview request and a confirmation
/// dialog by [`DeleteEntityModal`].
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DeleteTarget {
    pub id: String,
    pub name: String,
    pub kind: DeleteEntityKind,
}

impl DeleteTarget {
    pub fn list(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: DeleteEntityKind::List,
        }
    }

    pub fn container(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            kind: DeleteEntityKind::Container,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
enum PreviewState {
    #[default]
    Loading,
    Ready(DeletePreview),
}

fn tr_count(i18n: &I18n, key: &str, count: i64) -> String {
    let mut args = HashMap::new();
    args.insert("count".into(), count.into());
    i18n.tr_with_args(key, &args)
}

fn tr_two_counts(i18n: &I18n, key: &str, lists: i64, containers: i64) -> String {
    let mut args = HashMap::new();
    args.insert("lists".into(), lists.into());
    args.insert("containers".into(), containers.into());
    i18n.tr_with_args(key, &args)
}

fn title_for(i18n: &I18n, kind: DeleteEntityKind) -> String {
    i18n.tr(match kind {
        DeleteEntityKind::List => "lists-confirm-delete-title",
        DeleteEntityKind::Container => "lists-confirm-delete-container-title",
    })
}

fn confirm_label_for(i18n: &I18n, kind: DeleteEntityKind) -> String {
    i18n.tr(match kind {
        DeleteEntityKind::List => "lists-confirm-delete-button",
        DeleteEntityKind::Container => "lists-confirm-delete-container-button",
    })
}

/// "Are you sure…" sentence for the given kind and name.
fn plain_message_for(i18n: &I18n, kind: DeleteEntityKind, name: &str) -> String {
    let mut args = HashMap::new();
    args.insert("name".into(), name.to_string().into());
    i18n.tr_with_args(
        match kind {
            DeleteEntityKind::List => "lists-confirm-delete-message",
            DeleteEntityKind::Container => "lists-confirm-delete-container-message",
        },
        &args,
    )
}

/// Blocking sentence: something in the tree still holds items.
fn blocked_message_for(i18n: &I18n, kind: DeleteEntityKind, name: &str, items: i64) -> String {
    let mut args = HashMap::new();
    args.insert("name".into(), name.to_string().into());
    args.insert("count".into(), items.into());
    i18n.tr_with_args(
        match kind {
            DeleteEntityKind::List => "lists-confirm-delete-blocked",
            DeleteEntityKind::Container => "lists-confirm-delete-container-blocked",
        },
        &args,
    )
}

/// Sentence listing the empty descendants that go away with the entity.
fn descendants_message_for(i18n: &I18n, preview: DeletePreview) -> Option<String> {
    match (preview.lists, preview.containers) {
        (0, 0) => None,
        (lists, 0) => Some(tr_count(
            i18n,
            "lists-confirm-delete-descendant-lists",
            lists,
        )),
        (0, containers) => Some(tr_count(
            i18n,
            "lists-confirm-delete-descendant-containers",
            containers,
        )),
        (lists, containers) => Some(tr_two_counts(
            i18n,
            "lists-confirm-delete-descendant-both",
            lists,
            containers,
        )),
    }
}

/// What the dialog must look like for a preview.
struct Dialog {
    message: String,
    hide_confirm: bool,
}

fn dialog_for(i18n: &I18n, target: &DeleteTarget, preview: DeletePreview) -> Dialog {
    if preview.items > 0 {
        return Dialog {
            message: blocked_message_for(i18n, target.kind, &target.name, preview.items),
            hide_confirm: true,
        };
    }

    let message = match descendants_message_for(i18n, preview) {
        Some(descendants) => format!(
            "{} {}",
            plain_message_for(i18n, target.kind, &target.name),
            descendants
        ),
        None => plain_message_for(i18n, target.kind, &target.name),
    };

    Dialog {
        message,
        hide_confirm: false,
    }
}

/// The single delete-confirmation flow used by every list/container delete site.
///
/// Set `target` to ask for a deletion; the component fetches the preview, renders the
/// matching dialog, performs the delete and calls `on_deleted` on success so the caller
/// can refresh its view or navigate away. Success toasts stay with the caller, errors are
/// toasted here.
#[component]
pub fn DeleteEntityModal(
    target: RwSignal<Option<DeleteTarget>>,
    on_deleted: Callback<DeleteTarget>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let preview: RwSignal<PreviewState> = RwSignal::new(PreviewState::Loading);
    let deleting = RwSignal::new(false);

    // Fetch the preview whenever a different entity is opened.
    Effect::new(move |_| {
        let Some(requested) = target.get() else {
            return;
        };
        preview.set(PreviewState::Loading);
        deleting.set(false);
        let error_prefix = i18n.tr("lists-confirm-delete-preview-error");
        leptos::task::spawn_local(async move {
            let result = match requested.kind {
                DeleteEntityKind::List => delete_list_preview(requested.id.clone()).await,
                DeleteEntityKind::Container => delete_container_preview(requested.id.clone()).await,
            };
            // The dialog may have been closed or reopened for another entity meanwhile.
            if target.try_get_untracked().flatten().as_ref() != Some(&requested) {
                return;
            }
            match result {
                Ok(summary) => preview.set(PreviewState::Ready(summary)),
                Err(e) => {
                    toast.push(format!("{error_prefix} {e}"), ToastKind::Error);
                    target.set(None);
                }
            }
        });
    });

    let on_close = Callback::new(move |_| target.set(None));

    let on_confirm = Callback::new(move |_| {
        let Some(requested) = target.get_untracked() else {
            return;
        };
        if deleting.get_untracked() {
            return;
        }
        let PreviewState::Ready(summary) = preview.get_untracked() else {
            return;
        };
        // `force` is only needed when the (empty) tree has descendants.
        let force = summary.lists + summary.containers > 0;
        deleting.set(true);
        let request = requested.clone();
        leptos::task::spawn_local(async move {
            let result = match request.kind {
                DeleteEntityKind::List => delete_list(request.id.clone(), force).await,
                DeleteEntityKind::Container => delete_container(request.id.clone(), force).await,
            };
            deleting.set(false);
            match result {
                Ok(()) => {
                    target.set(None);
                    on_deleted.run(request);
                }
                Err(e) => {
                    target.set(None);
                    toast.push(e.to_string(), ToastKind::Error);
                }
            }
        });
    });

    view! {
        {move || {
            let requested = target.get()?;
            let state = preview.get();
            let title = title_for(&i18n, requested.kind);
            let confirm_label = confirm_label_for(&i18n, requested.kind);
            let busy = deleting.get();
            let (message, hide_confirm) = match &state {
                PreviewState::Loading => (i18n.tr("lists-confirm-delete-loading"), true),
                PreviewState::Ready(_) if busy => (i18n.tr("lists-confirm-delete-loading"), true),
                PreviewState::Ready(summary) => {
                    let dialog = dialog_for(&i18n, &requested, *summary);
                    (dialog.message, dialog.hide_confirm)
                }
            };
            Some(view! {
                <ConfirmModal
                    open=Signal::derive(move || target.get().is_some())
                    title=title
                    message=message
                    confirm_label=confirm_label
                    variant=ConfirmVariant::Danger
                    on_confirm=on_confirm
                    on_close=on_close
                    hide_confirm=hide_confirm
                />
            })
        }}
    }
}
