use leptos::ev::{KeyboardEvent, MouseEvent};
use leptos::prelude::*;

use super::icon::{Icon, IconName};
use super::icon_button::{IconButton, IconButtonSize};

/// One entry of an [`ActionMenu`].
#[derive(Clone)]
pub struct ActionMenuItem {
    /// Lucide icon drawn on the left of the label.
    pub icon: IconName,
    /// Visible label (and the item's accessible name).
    pub label: Signal<String>,
    /// Click handler; the menu closes before it runs.
    pub on_select: Callback<()>,
    /// Render the item in the destructive colour.
    pub danger: bool,
    /// Optional `data-testid` for the item button.
    pub testid: Option<&'static str>,
}

impl ActionMenuItem {
    /// Builds a non-destructive item with the given icon and label.
    pub fn new(
        icon: impl Into<IconName>,
        label: impl Into<Signal<String>>,
        on_select: Callback<()>,
    ) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            on_select,
            danger: false,
            testid: None,
        }
    }

    /// Marks the item as destructive (error colour).
    #[must_use]
    pub const fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// Sets the `data-testid` of the item button.
    #[must_use]
    pub const fn testid(mut self, testid: &'static str) -> Self {
        self.testid = Some(testid);
        self
    }
}

/// The single action menu shared by container cards, list cards and sublist
/// headers — the fix for the inconsistent card menus (issue #205).
///
/// The trigger is always a "more" [`IconButton`]. Visibility is driven by the
/// caller-owned `open` signal instead of DaisyUI's `:focus-within` rules, so
/// server-rendered and hydrated markup always agree; an invisible backdrop
/// handles clicks outside the menu and Escape closes it. Picking an item closes
/// the menu before running its callback.
#[component]
pub fn ActionMenu(
    /// Menu entries, rendered in order.
    items: Vec<ActionMenuItem>,
    /// Open state, owned by the caller so it can be closed on navigation etc.
    open: RwSignal<bool>,
    /// Accessible name of the trigger button.
    #[prop(into)]
    label: Signal<String>,
    /// Optional `data-testid` for the trigger button.
    #[prop(optional_no_strip)]
    testid: Option<&'static str>,
    /// Extra classes for the wrapping `relative` div (e.g. a z-index that lifts
    /// the trigger above a DaisyUI collapse overlay).
    #[prop(default = String::new(), into)]
    class: String,
    #[prop(optional)] size: IconButtonSize,
) -> impl IntoView {
    let items = StoredValue::new(items);

    view! {
        <div
            class=format!("relative {class}")
            // Clicks inside the menu (and on its click-away layer) never reach a card's
            // own click-to-open handler.
            on:click=move |ev: MouseEvent| ev.stop_propagation()
            on:keydown=move |ev: KeyboardEvent| {
                if ev.key() == "Escape" {
                    open.set(false);
                }
            }
        >
            <IconButton
                icon="more-horizontal"
                label=label
                testid=testid
                size=size
                on_click=Callback::new(move |_| open.update(|is_open| *is_open = !*is_open))
            />
            {move || {
                open.get().then(|| {
                    let entries = items.get_value();
                    view! {
                        // Invisible click-away layer below the menu, above the page.
                        <div
                            class="fixed inset-0 z-40 cursor-default"
                            data-testid="action-menu-backdrop"
                            on:click=move |_: MouseEvent| open.set(false)
                        />
                        <ul
                            class="menu absolute right-0 top-full mt-1 z-50 w-52 p-1 bg-base-100 rounded-box border border-base-300 shadow-lg"
                            data-testid="action-menu"
                        >
                            {entries
                                .into_iter()
                                .map(|item| {
                                    view! {
                                        <li>
                                            <button
                                                type="button"
                                                data-testid=item.testid
                                                class=if item.danger { "text-error" } else { "" }
                                                on:click=move |_| {
                                                    open.set(false);
                                                    item.on_select.run(());
                                                }
                                            >
                                                <Icon name=item.icon.clone() size=16 />
                                                <span>{item.label}</span>
                                            </button>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                })
            }}
        </div>
    }
}
