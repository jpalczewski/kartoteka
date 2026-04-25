use kartoteka_shared::types::Item;
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::items::date_field::DateFieldInput;
use crate::server_fns::items::create_item;

#[component]
pub fn AddItemInput(
    list_id: Signal<String>,
    #[prop(default = false)] has_quantity: bool,
    on_created: Callback<Item>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (title, set_title) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (quantity_str, set_quantity_str) = signal(String::new());
    let (unit, set_unit) = signal("szt.".to_string());
    let start_date = RwSignal::new(String::new());
    let deadline = RwSignal::new(String::new());
    let hard_deadline = RwSignal::new(String::new());
    let (show_dates, set_show_dates) = signal(false);
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
                    set_title.set(String::new());
                    set_description.set(String::new());
                    set_quantity_str.set(String::new());
                    set_unit.set("szt.".to_string());
                    start_date.set(String::new());
                    deadline.set(String::new());
                    hard_deadline.set(String::new());
                    set_show_dates.set(false);
                    on_created.run(item);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
            set_pending.set(false);
        });
    };

    view! {
        <form on:submit=on_submit class="flex flex-col gap-2">
            <div class="flex gap-2">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder="Nowy element..."
                    prop:value=move || title.get()
                    on:input=move |ev| set_title.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn-primary" prop:disabled=move || pending.get()>
                    "Dodaj"
                </button>
            </div>
            <textarea
                class="textarea textarea-bordered text-sm"
                placeholder="Opis (opcjonalnie)..."
                prop:value=move || description.get()
                on:input=move |ev| set_description.set(event_target_value(&ev))
            />
            {has_quantity.then(|| view! {
                <div class="flex gap-2">
                    <input
                        type="number"
                        class="input input-bordered input-sm w-24"
                        placeholder="Ilość"
                        min="1"
                        prop:value=move || quantity_str.get()
                        on:input=move |ev| set_quantity_str.set(event_target_value(&ev))
                    />
                    <input
                        type="text"
                        class="input input-bordered input-sm w-20"
                        placeholder="jedn."
                        prop:value=move || unit.get()
                        on:input=move |ev| set_unit.set(event_target_value(&ev))
                    />
                </div>
            })}

            <div>
                <button
                    type="button"
                    class="btn btn-ghost btn-xs text-base-content/50"
                    on:click=move |_| set_show_dates.update(|v| *v = !*v)
                >
                    {move || if show_dates.get() { "▲ Ukryj daty" } else { "＋ Daty" }}
                </button>

                {move || show_dates.get().then(|| view! {
                    <div class="flex flex-col gap-1 mt-2">
                        <DateFieldInput label="📅 Rozpoczęcie" value=start_date/>
                        <DateFieldInput label="⏰ Termin" value=deadline/>
                        <DateFieldInput label="🚨 Ostateczny" value=hard_deadline/>
                    </div>
                })}
            </div>
        </form>
    }
}
