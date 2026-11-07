use leptos::prelude::*;

use kartoteka_shared::ChangeTopic;

use crate::server_fns::time_entries::{
    assign_time_entry, delete_time_entry, get_inbox, list_all_entries,
};
use crate::state::invalidation::{mutate, use_invalidation};

fn format_duration(secs: Option<i32>) -> String {
    match secs {
        None => "Trwa…".to_string(),
        Some(s) => {
            let h = s / 3600;
            let m = (s % 3600) / 60;
            let sec = s % 60;
            if h > 0 {
                format!("{h}h {m:02}min")
            } else if m > 0 {
                format!("{m}min {sec:02}s")
            } else {
                format!("{sec}s")
            }
        }
    }
}

fn truncate_id(id: &str) -> &str {
    &id[..8.min(id.len())]
}

#[component]
pub fn TimePage() -> impl IntoView {
    let invalidation = use_invalidation();

    let inbox_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::TimeEntries]),
        move |_| get_inbox(),
    );
    let all_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::TimeEntries]),
        move |_| list_all_entries(),
    );

    let assign_inputs: RwSignal<std::collections::HashMap<String, String>> =
        RwSignal::new(std::collections::HashMap::new());

    let on_assign = move |entry_id: String| {
        let item_id = assign_inputs.with(|m| m.get(&entry_id).cloned().unwrap_or_default());
        if item_id.trim().is_empty() {
            return;
        }
        mutate(
            assign_time_entry(entry_id, item_id),
            &[ChangeTopic::TimeEntries],
        );
    };

    let on_delete = move |entry_id: String| {
        mutate(delete_time_entry(entry_id), &[ChangeTopic::TimeEntries]);
    };

    view! {
        <div class="container mx-auto max-w-3xl p-4">
            <h1 class="text-2xl font-bold mb-6">"Czas"</h1>

            <section class="mb-8">
                <h2 class="text-lg font-semibold mb-3">"Nieprzypisane wpisy (inbox)"</h2>
                <Suspense fallback=|| view! { <span class="loading loading-dots"></span> }>
                    {move || {
                        match inbox_res.get() {
                            Some(Ok(entries)) if entries.is_empty() => {
                                view! {
                                    <p class="text-base-content/40 italic">
                                        "Brak nieprzypisanych wpisów."
                                    </p>
                                }
                                    .into_any()
                            }
                            Some(Ok(entries)) => {
                                view! {
                                    <div class="overflow-x-auto">
                                        <table class="table table-sm">
                                            <thead>
                                                <tr>
                                                    <th>"Rozpoczęto"</th>
                                                    <th>"Czas"</th>
                                                    <th>"Opis"</th>
                                                    <th>"Przypisz do zadania"</th>
                                                    <th></th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {entries
                                                    .into_iter()
                                                    .map(|e| {
                                                        let eid = e.id.clone();
                                                        let eid2 = e.id.clone();
                                                        let eid3 = e.id.clone();
                                                        view! {
                                                            <tr>
                                                                <td class="font-mono text-xs">
                                                                    {e.started_at.clone()}
                                                                </td>
                                                                <td>{format_duration(e.duration)}</td>
                                                                <td>
                                                                    {e.description.clone().unwrap_or_default()}
                                                                </td>
                                                                <td>
                                                                    <div class="flex gap-1">
                                                                        <input
                                                                            type="text"
                                                                            class="input input-bordered input-xs w-32"
                                                                            placeholder="ID zadania…"
                                                                            prop:value=move || {
                                                                                assign_inputs
                                                                                    .with(|m| {
                                                                                        m.get(&eid).cloned().unwrap_or_default()
                                                                                    })
                                                                            }
                                                                            on:input=move |ev| {
                                                                                let val = event_target_value(&ev);
                                                                                assign_inputs
                                                                                    .update(|m| {
                                                                                        m.insert(eid2.clone(), val);
                                                                                    });
                                                                            }
                                                                        />
                                                                        <button
                                                                            type="button"
                                                                            class="btn btn-xs btn-primary"
                                                                            on:click=move |_| on_assign(eid3.clone())
                                                                        >
                                                                            "Przypisz"
                                                                        </button>
                                                                    </div>
                                                                </td>
                                                                <td>
                                                                    <button
                                                                        type="button"
                                                                        class="btn btn-xs btn-ghost text-error"
                                                                        on:click={
                                                                            let id = e.id.clone();
                                                                            move |_| on_delete(id.clone())
                                                                        }
                                                                    >
                                                                        "✕"
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                                    .into_any()
                            }
                            Some(Err(e)) => {
                                view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }
                                    .into_any()
                            }
                            None => view! {}.into_any(),
                        }
                    }}
                </Suspense>
            </section>

            <section>
                <h2 class="text-lg font-semibold mb-3">"Wszystkie wpisy"</h2>
                <Suspense fallback=|| view! { <span class="loading loading-dots"></span> }>
                    {move || {
                        match all_res.get() {
                            Some(Ok(entries)) if entries.is_empty() => {
                                view! {
                                    <p class="text-base-content/40 italic">"Brak wpisów."</p>
                                }
                                    .into_any()
                            }
                            Some(Ok(entries)) => {
                                view! {
                                    <div class="overflow-x-auto">
                                        <table class="table table-sm">
                                            <thead>
                                                <tr>
                                                    <th>"Zadanie"</th>
                                                    <th>"Rozpoczęto"</th>
                                                    <th>"Czas"</th>
                                                    <th>"Źródło"</th>
                                                    <th></th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {entries
                                                    .into_iter()
                                                    .map(|e| {
                                                        let item_label = e
                                                            .item_id
                                                            .as_deref()
                                                            .map(truncate_id)
                                                            .map(|s| s.to_string())
                                                            .unwrap_or_else(|| "—".to_string());
                                                        let id = e.id.clone();
                                                        view! {
                                                            <tr>
                                                                <td class="font-mono text-xs">
                                                                    {item_label}
                                                                </td>
                                                                <td class="font-mono text-xs">
                                                                    {e.started_at.clone()}
                                                                </td>
                                                                <td>{format_duration(e.duration)}</td>
                                                                <td>{e.source.clone()}</td>
                                                                <td>
                                                                    <button
                                                                        type="button"
                                                                        class="btn btn-xs btn-ghost text-error"
                                                                        on:click=move |_| on_delete(id.clone())
                                                                    >
                                                                        "✕"
                                                                    </button>
                                                                </td>
                                                            </tr>
                                                        }
                                                    })
                                                    .collect::<Vec<_>>()}
                                            </tbody>
                                        </table>
                                    </div>
                                }
                                    .into_any()
                            }
                            Some(Err(e)) => {
                                view! { <p class="text-error">"Błąd: " {e.to_string()}</p> }
                                    .into_any()
                            }
                            None => view! {}.into_any(),
                        }
                    }}
                </Suspense>
            </section>
        </div>
    }
}
