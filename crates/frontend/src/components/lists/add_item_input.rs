use kartoteka_shared::types::Item;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::app::{ToastContext, ToastKind};
use crate::components::items::date_field::DateFieldInput;
use crate::components::ui::Icon;
use crate::server_fns::items::create_item;

/// What the user has typed into an [`AddItemInput`] but not submitted yet.
///
/// A page that re-creates its content on refetch (the list page renders everything from
/// one resource closure) owns the draft, so a live update does not wipe the form (#426).
#[derive(Clone, Copy)]
pub struct AddItemDraft {
    pub title: RwSignal<String>,
    pub description: RwSignal<String>,
    pub quantity: RwSignal<String>,
    pub unit: RwSignal<String>,
    pub start_date: RwSignal<String>,
    pub deadline: RwSignal<String>,
    pub hard_deadline: RwSignal<String>,
    pub show_dates: RwSignal<bool>,
}

const DEFAULT_UNIT: &str = "szt.";

impl AddItemDraft {
    pub fn new() -> Self {
        Self {
            title: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            quantity: RwSignal::new(String::new()),
            unit: RwSignal::new(DEFAULT_UNIT.to_string()),
            start_date: RwSignal::new(String::new()),
            deadline: RwSignal::new(String::new()),
            hard_deadline: RwSignal::new(String::new()),
            show_dates: RwSignal::new(false),
        }
    }

    /// Reset every field to its empty state.
    pub fn clear(self) {
        self.title.set(String::new());
        self.description.set(String::new());
        self.quantity.set(String::new());
        self.unit.set(DEFAULT_UNIT.to_string());
        self.start_date.set(String::new());
        self.deadline.set(String::new());
        self.hard_deadline.set(String::new());
        self.show_dates.set(false);
    }
}

impl Default for AddItemDraft {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
pub fn AddItemInput(
    list_id: Signal<String>,
    #[prop(default = false)] has_quantity: bool,
    on_created: Callback<Item>,
    /// Draft owned by the caller; without it the form keeps its own state.
    #[prop(optional)]
    draft: Option<AddItemDraft>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let draft = draft.unwrap_or_default();
    let AddItemDraft {
        title,
        description,
        quantity: quantity_str,
        unit,
        start_date,
        deadline,
        hard_deadline,
        show_dates,
    } = draft;
    let (pending, set_pending) = signal(false);

    let on_submit = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let t = title.get_untracked().trim().to_string();
        if t.is_empty() {
            return;
        }
        let lid = list_id.get_untracked();
        let desc = {
            let d = description.get_untracked();
            if d.trim().is_empty() { None } else { Some(d) }
        };
        let qty = if has_quantity {
            quantity_str.get_untracked().trim().parse::<i32>().ok()
        } else {
            None
        };
        let u = if has_quantity && qty.is_some() {
            let u = unit.get_untracked();
            if u.trim().is_empty() { None } else { Some(u) }
        } else {
            None
        };
        let sd = {
            let v = start_date.get_untracked();
            if v.is_empty() { None } else { Some(v) }
        };
        let dl = {
            let v = deadline.get_untracked();
            if v.is_empty() { None } else { Some(v) }
        };
        let hd = {
            let v = hard_deadline.get_untracked();
            if v.is_empty() { None } else { Some(v) }
        };
        set_pending.set(true);
        leptos::task::spawn_local(async move {
            match create_item(lid, t, desc, qty, u, sd, dl, hd).await {
                Ok(item) => {
                    draft.clear();
                    on_created.run(item);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
            // The form may have been re-created meanwhile (the draft outlives it).
            _ = set_pending.try_set(false);
        });
    };

    view! {
        <form on:submit=on_submit class="flex flex-col gap-2" data-testid="add-item-form">
            <div class="flex gap-2">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    data-testid="add-item-title"
                    placeholder="Nowy element..."
                    prop:value=move || title.get()
                    on:input=move |ev| title.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary" prop:disabled=move || pending.get()>
                    "Dodaj"
                </button>
            </div>
            <textarea
                class="textarea textarea-bordered text-sm"
                placeholder="Opis (opcjonalnie)..."
                prop:value=move || description.get()
                on:input=move |ev| description.set(event_target_value(&ev))
            />
            {has_quantity.then(|| view! {
                <div class="flex gap-2">
                    <input
                        type="number"
                        class="input input-bordered input-sm w-24"
                        placeholder="Ilość"
                        min="1"
                        prop:value=move || quantity_str.get()
                        on:input=move |ev| quantity_str.set(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        class="input input-bordered input-sm w-20"
                        placeholder="jedn."
                        prop:value=move || unit.get()
                        on:input=move |ev| unit.set(event_target_value(&ev))
                    />
                </div>
            })}

            <div>
                <button
                    type="button"
                    class="btn btn-ghost btn-xs text-base-content/50"
                    data-testid="add-item-dates-toggle"
                    on:click=move |_| show_dates.update(|v| *v = !*v)
                >
                    <Icon
                        name=Signal::derive(move || {
                            if show_dates.get() { "chevron-up".to_string() } else { "plus".to_string() }
                        })
                        size=14
                    />
                    {if show_dates.get() { "Ukryj daty" } else { "Daty" }}
                </button>

                {move || show_dates.get().then(|| view! {
                    <div class="flex flex-col gap-1 mt-2">
                        <DateFieldInput label=move_tr!("item-dates-start") value=start_date/>
                        <DateFieldInput label=move_tr!("item-dates-deadline") value=deadline data_testid="add-item-deadline"/>
                        <DateFieldInput label=move_tr!("item-dates-hard-deadline") value=hard_deadline/>
                    </div>
                })}
            </div>
        </form>
    }
}
