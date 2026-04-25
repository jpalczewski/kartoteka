use kartoteka_shared::types::Relation;
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::server_fns::relations::{add_relation, get_relations, remove_relation};

fn relation_label(rel: &Relation, current_entity_id: &str) -> String {
    let is_outgoing = rel.from_id == current_entity_id;
    match (rel.relation_type.as_str(), is_outgoing) {
        ("blocks", true) => format!("Blokuje → {}", &rel.to_id[..8.min(rel.to_id.len())]),
        ("blocks", false) => format!(
            "Zablokowane przez ← {}",
            &rel.from_id[..8.min(rel.from_id.len())]
        ),
        ("relates_to", true) => format!("Powiązane → {}", &rel.to_id[..8.min(rel.to_id.len())]),
        ("relates_to", false) => {
            format!("Powiązane ← {}", &rel.from_id[..8.min(rel.from_id.len())])
        }
        (kind, true) => format!("{kind} → {}", &rel.to_id[..8.min(rel.to_id.len())]),
        (kind, false) => format!("{kind} ← {}", &rel.from_id[..8.min(rel.from_id.len())]),
    }
}

#[component]
pub fn RelatedEntities(entity_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let target_id = RwSignal::new(String::new());
    let rel_type = RwSignal::new("blocks".to_string());

    let rels_res = Resource::new(
        move || (entity_id.get(), refresh.get()),
        move |(eid, _)| get_relations("item".to_string(), eid),
    );

    let on_delete = Callback::new(move |relation_id: String| {
        leptos::task::spawn_local(async move {
            match remove_relation(relation_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_add = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        let tid = target_id.get();
        if tid.trim().is_empty() {
            return;
        }
        let from_id = entity_id.get();
        let rtype = rel_type.get();
        leptos::task::spawn_local(async move {
            match add_relation("item".to_string(), from_id, "item".to_string(), tid, rtype).await {
                Ok(_) => {
                    target_id.set(String::new());
                    set_refresh.update(|n| *n += 1);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="mt-6">
            <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-3">
                "Powiązania"
            </h3>

            <Suspense fallback=|| view! { <span class="loading loading-dots loading-xs"></span> }>
                {move || {
                    let current_eid = entity_id.get();
                    match rels_res.get() {
                        Some(Ok(rels)) if rels.is_empty() => view! {
                            <p class="text-base-content/40 text-sm italic py-2">"Brak powiązań."</p>
                        }
                        .into_any(),
                        Some(Ok(rels)) => {
                            let eid = current_eid.clone();
                            view! {
                                <div class="flex flex-col gap-2">
                                    {rels
                                        .into_iter()
                                        .map(|rel| {
                                            let label = relation_label(&rel, &eid);
                                            let rid = rel.id.clone();
                                            view! {
                                                <div class="flex items-center justify-between rounded-lg bg-base-200 px-3 py-2">
                                                    <span class="text-sm">{label}</span>
                                                    <button
                                                        type="button"
                                                        class="btn btn-ghost btn-xs btn-circle text-error"
                                                        on:click=move |_| on_delete.run(rid.clone())
                                                    >
                                                        {"✕"}
                                                    </button>
                                                </div>
                                            }
                                        })
                                        .collect::<Vec<_>>()}
                                </div>
                            }
                            .into_any()
                        }
                        Some(Err(e)) => view! {
                            <p class="text-error text-sm">"Błąd: " {e.to_string()}</p>
                        }
                        .into_any(),
                        None => view! {}.into_any(),
                    }
                }}
            </Suspense>

            <div class="flex gap-2 mt-3">
                <select
                    class="select select-bordered select-sm"
                    on:change=move |ev| rel_type.set(event_target_value(&ev))
                >
                    <option value="blocks">"Blokuje"</option>
                    <option value="relates_to">"Powiązane z"</option>
                </select>
                <input
                    type="text"
                    class="input input-bordered input-sm flex-1"
                    placeholder="ID zadania..."
                    prop:value=target_id
                    on:input=move |ev| target_id.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    class="btn btn-primary btn-sm"
                    disabled=move || target_id.get().trim().is_empty()
                    on:click=on_add
                >
                    "Dodaj"
                </button>
            </div>
        </div>
    }
}
