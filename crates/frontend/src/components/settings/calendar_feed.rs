//! Settings card for the iCal feed: subscription links and default reminders.
//!
//! The feed URL is built on the client from `window.location().origin()` and is
//! shown exactly once, right after a token is created — the server only ever
//! stores a hash of it.

use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::calendar_feed::{
    create_calendar_token_sf, get_reminder_defaults_sf, list_calendar_tokens_sf,
    revoke_calendar_token_sf, set_reminder_defaults_sf,
};
use crate::server_fns::lists::get_all_lists;
use kartoteka_shared::ReminderDefaults;
use kartoteka_shared::types::{CalendarTokenCreatedDto, CalendarTokenInfo};

/// Reminder offsets offered in the UI: minutes before the event date, with the
/// i18n key of their label.
const REMINDER_PRESETS: [(u32, &str); 8] = [
    (5, "settings-calendar-reminder-5m"),
    (15, "settings-calendar-reminder-15m"),
    (30, "settings-calendar-reminder-30m"),
    (60, "settings-calendar-reminder-1h"),
    (120, "settings-calendar-reminder-2h"),
    (1440, "settings-calendar-reminder-1d"),
    (2880, "settings-calendar-reminder-2d"),
    (10080, "settings-calendar-reminder-1w"),
];

/// Mirrors the domain rule: at most 5 offsets per date field.
const MAX_REMINDERS_PER_FIELD: usize = 5;

/// Both flavours of a feed address, built once when the token is created.
#[derive(Clone, PartialEq, Eq)]
struct FeedUrls {
    feed: String,
    webcal: String,
}

#[component]
pub fn CalendarFeedSettings() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (tokens_refresh, set_tokens_refresh) = signal(0u32);
    let tokens_res = Resource::new(move || tokens_refresh.get(), |_| list_calendar_tokens_sf());
    let reminders_res = Resource::new(|| (), |_| get_reminder_defaults_sf());
    let lists_res = Resource::new(|| (), |_| get_all_lists());

    let (new_name, set_new_name) = signal(String::new());
    let (new_list_id, set_new_list_id) = signal(String::new());
    let (created_urls, set_created_urls) = signal(Option::<FeedUrls>::None);

    let on_revoked = Callback::new(move |_: ()| set_tokens_refresh.update(|n| *n += 1));

    let on_create = move |_| {
        let name = new_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        let list_id = {
            let selected = new_list_id.get_untracked();
            if selected.is_empty() {
                None
            } else {
                Some(selected)
            }
        };
        leptos::task::spawn_local(async move {
            match create_calendar_token_sf(name, list_id).await {
                Ok(CalendarTokenCreatedDto { token, .. }) => {
                    set_created_urls.set(Some(feed_urls(&client_origin(), &token)));
                    set_new_name.set(String::new());
                    set_tokens_refresh.update(|n| *n += 1);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="card bg-base-200 p-4" data-testid="calendar-feed-settings">
            <h3 class="font-semibold mb-1">{move_tr!("settings-calendar-title")}</h3>
            <p class="text-sm text-base-content/60 mb-3">
                {move_tr!("settings-calendar-description")}
            </p>

            {move || created_urls.get().map(|urls| view! {
                <div class="alert alert-success alert-soft mb-3">
                    <div class="w-full">
                        <p class="font-semibold">{move_tr!("settings-calendar-created-title")}</p>
                        <p class="text-xs opacity-80 mt-1">
                            {move_tr!("settings-calendar-created-hint")}
                        </p>
                        <div class="mt-2 flex items-center gap-2">
                            <code class="flex-1 break-all rounded bg-base-300 px-2 py-1 text-xs">
                                {urls.feed.clone()}
                            </code>
                            <CopyButton text=urls.feed.clone()/>
                        </div>
                        <div class="mt-1 flex items-center gap-2">
                            <code class="flex-1 break-all rounded bg-base-300 px-2 py-1 text-xs">
                                {urls.webcal.clone()}
                            </code>
                            <CopyButton text=urls.webcal.clone()/>
                        </div>
                        <button
                            type="button"
                            class="btn btn-xs btn-outline mt-2"
                            on:click=move |_| set_created_urls.set(None)
                        >
                            {move_tr!("common-close")}
                        </button>
                    </div>
                </div>
            })}

            <Transition fallback=move || view! { <LoadingSpinner/> }>
                <div class="flex flex-col gap-2 sm:flex-row">
                    <input
                        type="text"
                        class="input input-bordered input-sm w-full sm:flex-1"
                        data-testid="calendar-feed-name"
                        placeholder=move_tr!("settings-calendar-name-placeholder")
                        prop:value=move || new_name.get()
                        on:input=move |ev| set_new_name.set(event_target_value(&ev))
                    />
                    <select
                        class="select select-bordered select-sm w-full sm:w-auto"
                        data-testid="calendar-feed-scope"
                        on:change=move |ev| set_new_list_id.set(event_target_value(&ev))
                    >
                        <option value="" selected=move || new_list_id.get().is_empty()>
                            {move_tr!("settings-calendar-scope-all")}
                        </option>
                        {move || {
                            lists_res
                                .get()
                                .and_then(Result::ok)
                                .unwrap_or_default()
                                .into_iter()
                                .map(|list| view! {
                                    <option value=list.id.clone()>{list.name.clone()}</option>
                                })
                                .collect::<Vec<_>>()
                        }}
                    </select>
                    <button
                        type="button"
                        class="btn btn-primary btn-sm"
                        data-testid="calendar-feed-create"
                        disabled=move || new_name.get().trim().is_empty()
                        on:click=on_create
                    >
                        {move_tr!("settings-calendar-create")}
                    </button>
                </div>

                <div class="mt-4">
                    <h4 class="font-semibold text-sm mb-1">
                        {move_tr!("settings-calendar-tokens-title")}
                    </h4>
                    {move || {
                        let tokens = tokens_res.get().and_then(Result::ok).unwrap_or_default();
                        if tokens.is_empty() {
                            return view! {
                                <p class="text-sm text-base-content/50">
                                    {move_tr!("settings-calendar-tokens-empty")}
                                </p>
                            }
                                .into_any();
                        }
                        view! {
                            <div class="flex flex-col gap-2">
                                {tokens
                                    .into_iter()
                                    .map(|token| view! {
                                        <CalendarTokenRow token on_revoked=on_revoked/>
                                    })
                                    .collect::<Vec<_>>()}
                            </div>
                        }
                            .into_any()
                    }}
                </div>

                <div class="mt-6">
                    <h4 class="font-semibold text-sm mb-1">
                        {move_tr!("settings-calendar-reminders-title")}
                    </h4>
                    <p class="text-xs text-base-content/60 mb-2">
                        {move_tr!("settings-calendar-reminders-description")}
                    </p>
                    <p class="text-xs text-base-content/60 mb-2">
                        {move_tr!("settings-calendar-reminders-allday-hint")}
                    </p>
                    {move || match reminders_res.get() {
                        Some(Ok(defaults)) => view! {
                            <ReminderDefaultsForm initial=defaults/>
                        }.into_any(),
                        Some(Err(e)) => view! {
                            <p class="text-sm text-error">{e.to_string()}</p>
                        }.into_any(),
                        None => view! {}.into_any(),
                    }}
                </div>
            </Transition>
        </div>
    }
}

/// One subscription token: name, scope, usage timestamps and a two-step revoke.
#[component]
fn CalendarTokenRow(token: CalendarTokenInfo, on_revoked: Callback<()>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();

    let (confirming, set_confirming) = signal(false);
    let token_id = StoredValue::new(token.id);
    let name = token.name;
    let scope = match (token.list_name, token.list_id) {
        (Some(list_name), _) => list_name,
        (None, Some(_)) => i18n.tr("settings-calendar-scope-missing-list"),
        (None, None) => i18n.tr("settings-calendar-scope-all"),
    };
    let last_used = token
        .last_used_at
        .unwrap_or_else(|| i18n.tr("settings-calendar-never-used"));
    let created_at = token.created_at;

    view! {
        <div class="flex items-start justify-between gap-2 p-2 bg-base-300 rounded">
            <div class="min-w-0">
                <div class="flex flex-wrap items-center gap-2">
                    <span class="font-medium text-sm">{name}</span>
                    <span class="badge badge-ghost badge-sm">{scope}</span>
                </div>
                <p class="text-xs text-base-content/50 mt-1">
                    {move_tr!("settings-calendar-last-used")} ": " {last_used}
                </p>
                <p class="text-xs text-base-content/50">
                    {move_tr!("common-created")} ": " {created_at}
                </p>
            </div>
            <div class="flex shrink-0 flex-col items-end gap-1">
                <Show
                    when=move || confirming.get()
                    fallback=move || view! {
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs text-error"
                            on:click=move |_| set_confirming.set(true)
                        >
                            {move_tr!("settings-calendar-revoke")}
                        </button>
                    }
                >
                    <button
                        type="button"
                        class="btn btn-error btn-xs"
                        on:click=move |_| {
                            let id = token_id.get_value();
                            leptos::task::spawn_local(async move {
                                match revoke_calendar_token_sf(id).await {
                                    Ok(()) => {
                                        set_confirming.set(false);
                                        on_revoked.run(());
                                    }
                                    Err(e) => {
                                        set_confirming.set(false);
                                        toast.push(e.to_string(), ToastKind::Error);
                                    }
                                }
                            });
                        }
                    >
                        {move_tr!("common-confirm")}
                    </button>
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs"
                        on:click=move |_| set_confirming.set(false)
                    >
                        {move_tr!("common-cancel")}
                    </button>
                </Show>
            </div>
        </div>
    }
}

/// Default reminders for one date field, picked from preset offsets.
#[component]
fn ReminderRow(label: Signal<String>, value: RwSignal<Vec<u32>>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();

    // Offsets set outside the UI (e.g. through the API) stay visible, so saving
    // from this form cannot silently drop them.
    let options: Vec<(u32, Signal<String>)> = {
        let mut options: Vec<(u32, Signal<String>)> = REMINDER_PRESETS
            .iter()
            .map(|(minutes, key)| (*minutes, Signal::derive(move || i18n.tr(key))))
            .collect();
        let mut extras: Vec<u32> = value
            .get_untracked()
            .into_iter()
            .filter(|minutes| !REMINDER_PRESETS.iter().any(|(preset, _)| preset == minutes))
            .collect();
        extras.sort_unstable();
        for minutes in extras {
            options.push((
                minutes,
                move_tr!(
                    "settings-calendar-reminder-minutes",
                    { "minutes" => minutes as usize }
                ),
            ));
        }
        options.sort_unstable_by_key(|(minutes, _)| *minutes);
        options
    };

    let limit_message = i18n.tr("settings-calendar-reminders-limit");

    view! {
        <div class="flex flex-col gap-1">
            <span class="text-sm font-medium">{label}</span>
            <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
                <label class="label cursor-pointer gap-1 px-1 py-0">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-xs"
                        prop:checked=move || value.get().is_empty()
                        on:change=move |_| value.set(Vec::new())
                    />
                    <span class="text-xs">{move_tr!("settings-calendar-reminders-none")}</span>
                </label>
                {options
                    .into_iter()
                    .map(|(minutes, option_label)| {
                        let limit_message = limit_message.clone();
                        view! {
                        <label class="label cursor-pointer gap-1 px-1 py-0">
                            <input
                                type="checkbox"
                                class="checkbox checkbox-xs"
                                prop:checked=move || value.get().contains(&minutes)
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    let mut overflow = false;
                                    value
                                        .update(|offsets| {
                                            if checked {
                                                if !offsets.contains(&minutes) {
                                                    if offsets.len() >= MAX_REMINDERS_PER_FIELD {
                                                        offsets.remove(0);
                                                        overflow = true;
                                                    }
                                                    offsets.push(minutes);
                                                }
                                            } else {
                                                offsets.retain(|offset| *offset != minutes);
                                            }
                                        });
                                    if overflow {
                                        toast.push(limit_message.clone(), ToastKind::Error);
                                    }
                                }
                            />
                            <span class="text-xs">{option_label}</span>
                        </label>
                        }
                    })
                    .collect::<Vec<_>>()}
            </div>
        </div>
    }
}

/// Three reminder rows plus the save action.
#[component]
fn ReminderDefaultsForm(initial: ReminderDefaults) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();

    let start_date = RwSignal::new(initial.start_date);
    let deadline = RwSignal::new(initial.deadline);
    let hard_deadline = RwSignal::new(initial.hard_deadline);

    let (saving, set_saving) = signal(false);
    let (feedback, set_feedback) = signal(Option::<bool>::None);
    let saved_message = i18n.tr("settings-calendar-reminders-saved");

    let on_save = move |_| {
        let value = ReminderDefaults {
            start_date: sorted_offsets(start_date.get_untracked()),
            deadline: sorted_offsets(deadline.get_untracked()),
            hard_deadline: sorted_offsets(hard_deadline.get_untracked()),
        };
        let saved_message = saved_message.clone();
        set_saving.set(true);
        set_feedback.set(None);
        leptos::task::spawn_local(async move {
            match set_reminder_defaults_sf(value).await {
                Ok(()) => {
                    set_saving.set(false);
                    set_feedback.set(Some(true));
                    toast.push(saved_message, ToastKind::Success);
                }
                Err(e) => {
                    set_saving.set(false);
                    set_feedback.set(Some(false));
                    toast.push(e.to_string(), ToastKind::Error);
                }
            }
        });
    };

    view! {
        <div class="flex flex-col gap-3">
            <ReminderRow label=move_tr!("settings-calendar-reminders-start") value=start_date/>
            <ReminderRow label=move_tr!("settings-calendar-reminders-deadline") value=deadline/>
            <ReminderRow
                label=move_tr!("settings-calendar-reminders-hard-deadline")
                value=hard_deadline
            />
            <div class="flex items-center gap-3">
                <button
                    type="button"
                    class="btn btn-primary btn-sm"
                    data-testid="calendar-reminders-save"
                    disabled=move || saving.get()
                    on:click=on_save
                >
                    {move_tr!("common-save")}
                </button>
                {move || match feedback.get() {
                    Some(true) => view! {
                        <span class="text-sm text-success">
                            {move_tr!("settings-calendar-reminders-saved")}
                        </span>
                    }.into_any(),
                    Some(false) => view! {
                        <span class="text-sm text-error">
                            {move_tr!("settings-calendar-reminders-error")}
                        </span>
                    }.into_any(),
                    None => view! {}.into_any(),
                }}
            </div>
        </div>
    }
}

/// Copy-to-clipboard button for a feed address.
#[component]
fn CopyButton(text: String) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let copied_message = expect_context::<I18n>().tr("common-copied");

    view! {
        <button
            type="button"
            class="btn btn-xs btn-neutral shrink-0"
            title=move_tr!("common-copy")
            on:click=move |_| {
                copy_to_clipboard(&text);
                toast.push(copied_message.clone(), ToastKind::Success);
            }
        >
            {move_tr!("common-copy")}
        </button>
    }
}

/// Ascending, de-duplicated offsets so the stored set is independent of click order.
fn sorted_offsets(mut offsets: Vec<u32>) -> Vec<u32> {
    offsets.sort_unstable();
    offsets.dedup();
    offsets
}

/// The browser origin (`https://kartoteka.example.com`), empty during SSR. Feed
/// URLs are only ever rendered after a client-side create action.
fn client_origin() -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(origin) = window.location().origin() {
            return origin;
        }
    }
    String::new()
}

/// Feed URLs for a token: the HTTPS address and the same address with a
/// `webcal://` scheme, which calendar apps register as a subscription.
fn feed_urls(origin: &str, token: &str) -> FeedUrls {
    let base = origin.trim_end_matches('/');
    let host = base
        .strip_prefix("https://")
        .or_else(|| base.strip_prefix("http://"))
        .unwrap_or(base);
    FeedUrls {
        feed: format!("{base}/cal/{token}/feed.ics"),
        webcal: format!("webcal://{host}/cal/{token}/feed.ics"),
    }
}

/// Writes `text` to the system clipboard. A no-op during SSR.
#[allow(unused_variables)]
fn copy_to_clipboard(text: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        // Fire and forget: the promise only reports whether the write succeeded.
        let _ = window.navigator().clipboard().write_text(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_urls_use_https_and_webcal_schemes() {
        let urls = feed_urls("https://kartoteka.example.com", "abc123");
        assert_eq!(
            urls.feed,
            "https://kartoteka.example.com/cal/abc123/feed.ics"
        );
        assert_eq!(
            urls.webcal,
            "webcal://kartoteka.example.com/cal/abc123/feed.ics"
        );
    }

    #[test]
    fn feed_urls_handle_local_http_origin() {
        let urls = feed_urls("http://localhost:3000", "abc123");
        assert_eq!(urls.feed, "http://localhost:3000/cal/abc123/feed.ics");
        assert_eq!(urls.webcal, "webcal://localhost:3000/cal/abc123/feed.ics");
    }

    #[test]
    fn sorted_offsets_are_ascending_and_unique() {
        assert_eq!(sorted_offsets(vec![60, 5, 60, 1440]), vec![5, 60, 1440]);
        assert!(sorted_offsets(vec![]).is_empty());
    }
}
