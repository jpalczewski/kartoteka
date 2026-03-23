use kartoteka_shared::List;
use leptos::prelude::*;

#[component]
pub fn ListCard(list: List) -> impl IntoView {
    let href = format!("/lists/{}", list.id);
    view! {
        <a href=href style="text-decoration: none; color: inherit;">
            <div class="card">
                <h3>{list.name}</h3>
                <span class="meta">{format!("{:?}", list.list_type)}</span>
            </div>
        </a>
    }
}
