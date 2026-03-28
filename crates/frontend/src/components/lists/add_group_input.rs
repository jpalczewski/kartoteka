use leptos::prelude::*;
use leptos_fluent::move_tr;

#[component]
pub fn AddGroupInput(on_submit: Callback<String>) -> impl IntoView {
    let adding = RwSignal::new(false);
    let name = RwSignal::new(String::new());

    let submit = move || {
        let val = name.get();
        if !val.trim().is_empty() {
            on_submit.run(val);
            name.set(String::new());
            adding.set(false);
        }
    };

    view! {
        <div class="mt-4">
            {move || {
                if adding.get() {
                    let submit_enter = submit.clone();
                    let submit_btn = submit.clone();
                    view! {
                        <div class="flex gap-2">
                            <input
                                type="text"
                                class="input input-bordered flex-1"
                                placeholder=move_tr!("lists-add-group-placeholder")
                                prop:value=name
                                on:input=move |ev| name.set(event_target_value(&ev))
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        submit_enter();
                                    } else if ev.key() == "Escape" {
                                        adding.set(false);
                                        name.set(String::new());
                                    }
                                }
                            />
                            <button
                                type="button"
                                class="btn btn-primary"
                                on:click=move |_| {
                                    submit_btn();
                                }
                            >
                                {move_tr!("common-add")}
                            </button>
                            <button
                                type="button"
                                class="btn btn-ghost"
                                on:click=move |_| {
                                    adding.set(false);
                                    name.set(String::new());
                                }
                            >
                                {move_tr!("common-cancel")}
                            </button>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <button
                            type="button"
                            class="btn btn-ghost btn-sm"
                            on:click=move |_| adding.set(true)
                        >
                            {move_tr!("lists-add-group-button")}
                        </button>
                    }.into_any()
                }
            }}
        </div>
    }
}
