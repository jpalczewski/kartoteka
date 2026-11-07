use kartoteka_shared::item_display::item_href;
use kartoteka_shared::types::Item;
use leptos::prelude::*;
use leptos_router::components::A;

/// Item title as a link to its detail page, with one completed/open style.
#[component]
pub fn ItemTitle(
    #[prop(into)] item: Signal<Option<Item>>,
    #[prop(optional)] compact: bool,
    /// Extra small text for narrow cells (calendar week view). Wins over `compact`.
    #[prop(optional)]
    dense: bool,
    #[prop(optional_no_strip)] testid: Option<&'static str>,
    #[prop(optional)] flex: bool,
) -> impl IntoView {
    let present = Memo::new(move |_| item.with(Option::is_some));
    let size_class = if dense {
        "text-xs leading-tight"
    } else if compact {
        "text-sm"
    } else {
        "text-base"
    };
    let flex_class = if flex { " flex-1" } else { "" };
    let href = move || item.with(|i| i.as_ref().map(item_href).unwrap_or_default());
    let title = move || item.with(|i| i.as_ref().map(|i| i.title.clone()).unwrap_or_default());
    let completed = move || item.with(|i| i.as_ref().is_some_and(|i| i.completed));

    move || {
        present.get().then(|| {
            view! {
                <A
                    href=href
                    attr:class=move || {
                        let style = if completed() {
                            "text-base-content/50 line-through"
                        } else {
                            "text-base-content hover:text-primary"
                        };
                        format!("{size_class}{flex_class} {style}")
                    }
                    attr:data-testid=testid
                >
                    {title}
                </A>
            }
        })
    }
}
