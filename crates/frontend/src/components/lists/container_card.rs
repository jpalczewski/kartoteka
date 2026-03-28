use kartoteka_shared::{Container, ContainerStatus};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::hooks::use_navigate;

pub fn container_icon(status: &Option<ContainerStatus>) -> &'static str {
    match status {
        None => "📁",
        Some(ContainerStatus::Active) => "🚀",
        Some(ContainerStatus::Done) => "✅",
        Some(ContainerStatus::Paused) => "⏸️",
    }
}

pub fn container_status_class(status: &ContainerStatus) -> &'static str {
    match status {
        ContainerStatus::Active => "badge badge-success badge-sm",
        ContainerStatus::Done => "badge badge-neutral badge-sm",
        ContainerStatus::Paused => "badge badge-warning badge-sm",
    }
}

#[component]
pub fn ContainerCard(
    container: Container,
    #[prop(optional)] completed_items: Option<u32>,
    #[prop(optional)] total_items: Option<u32>,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_pin: Option<Callback<String>>,
) -> impl IntoView {
    let href = format!("/containers/{}", container.id);
    let icon = container_icon(&container.status);
    let is_project = container.status.is_some();
    let is_pinned = container.pinned;

    let navigate = use_navigate();
    let href_clone = href.clone();

    let container_id_delete = container.id.clone();
    let container_id_pin = container.id.clone();

    let status_badge = container.status.as_ref().map(|s| {
        let cls = container_status_class(s);
        let label = match s {
            ContainerStatus::Active => move_tr!("lists-container-status-active"),
            ContainerStatus::Done => move_tr!("lists-container-status-done"),
            ContainerStatus::Paused => move_tr!("lists-container-status-paused"),
        };
        view! { <span class=cls>{label}</span> }
    });

    let progress_bar = if is_project {
        let total = total_items.unwrap_or(0);
        let completed = completed_items.unwrap_or(0);
        let pct = if total > 0 {
            (completed as f32 / total as f32 * 100.0) as u32
        } else {
            0
        };
        view! {
            <div class="mt-2">
                <div class="flex justify-between text-xs text-base-content/60 mb-1">
                    <span>{move_tr!("lists-tasks-progress", { "completed" => completed, "total" => total })}</span>
                    <span>{format!("{}%", pct)}</span>
                </div>
                <progress class="progress progress-primary w-full" value=completed max=total></progress>
            </div>
        }.into_any()
    } else {
        view! {}.into_any()
    };

    view! {
        <div
            class="card bg-base-200 border border-base-300 cursor-pointer card-neon relative"
            on:click=move |_| { navigate(&href_clone, Default::default()); }
        >
            // Pin button
            {on_pin.map(|cb| {
                let cid = container_id_pin.clone();
                let pin_icon = if is_pinned { "📌" } else { "📍" };
                view! {
                    <button
                        type="button"
                        aria-label={move_tr!("lists-pin-container-aria")}
                        class="btn btn-ghost btn-xs absolute top-2 right-8 opacity-40 hover:opacity-100"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(cid.clone());
                        }
                    >
                        {pin_icon}
                    </button>
                }
            })}

            // Delete button
            {on_delete.map(|cb| {
                let cid = container_id_delete.clone();
                view! {
                    <button
                        type="button"
                        aria-label={move_tr!("lists-delete-container-aria")}
                        class="btn btn-ghost btn-xs absolute top-2 right-2 opacity-40 hover:opacity-100"
                        on:click=move |ev| {
                            ev.stop_propagation();
                            cb.run(cid.clone());
                        }
                    >
                        "\u{1F5D1}"
                    </button>
                }
            })}

            <div class="card-body p-4">
                <div class="flex items-center gap-2">
                    <span class="text-lg">{icon}</span>
                    <h3 class="card-title text-base">{container.name.clone()}</h3>
                    {status_badge}
                    {if is_pinned {
                        view! { <span class="text-xs opacity-50">"📌"</span> }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
                {container.description.as_ref().map(|d| view! {
                    <p class="text-sm text-base-content/60 mt-1">{d.clone()}</p>
                })}
                {progress_bar}
            </div>
        </div>
    }
}
