use kartoteka_shared::{IconValue, parse_icon_value};
use leptos::ev::KeyboardEvent;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use super::icon::{Icon, icon_exists};

/// Lucide icons offered by the picker, all present in the local icon table.
///
/// UI chrome (chevrons, arrows, menu dots, …) is left out on purpose; only
/// icons that make sense as the face of a list or folder are listed.
const PICKER_ICONS: &[&str] = &[
    "list",
    "list-todo",
    "clipboard-list",
    "square-check",
    "circle-check",
    "notebook",
    "file-text",
    "newspaper",
    "book",
    "book-open",
    "bookmark",
    "folder",
    "folder-kanban",
    "folder-open",
    "archive",
    "inbox",
    "package",
    "package-open",
    "briefcase",
    "building",
    "school",
    "hospital",
    "store",
    "house",
    "sofa",
    "bed",
    "lamp",
    "key",
    "lock",
    "shield",
    "calendar",
    "calendar-days",
    "clock",
    "alarm-clock",
    "timer",
    "hourglass",
    "bell",
    "flag",
    "target",
    "star",
    "heart",
    "sparkles",
    "crown",
    "gem",
    "trophy",
    "gift",
    "flame",
    "zap",
    "lightbulb",
    "rocket",
    "compass",
    "map",
    "map-pin",
    "globe",
    "plane",
    "train-front",
    "car",
    "bike",
    "truck",
    "anchor",
    "luggage",
    "tent",
    "mountain",
    "tree-pine",
    "leaf",
    "flower-2",
    "sun",
    "moon",
    "cloud",
    "umbrella",
    "droplet",
    "thermometer",
    "shopping-cart",
    "shopping-bag",
    "wallet",
    "credit-card",
    "banknote",
    "piggy-bank",
    "shirt",
    "scissors",
    "hammer",
    "wrench",
    "paintbrush",
    "palette",
    "code",
    "bug",
    "mail",
    "phone",
    "camera",
    "image",
    "film",
    "music",
    "gamepad-2",
    "dumbbell",
    "utensils",
    "coffee",
    "beer",
    "pizza",
    "apple",
    "cake",
    "stethoscope",
    "pill",
    "baby",
    "dog",
    "cat",
    "smile",
    "user",
    "users",
    "graduation-cap",
    "tag",
    "pin",
    "link",
    "layout-grid",
    "columns-3",
    "repeat",
    "settings",
];

/// Emoji offered on the emoji tab; anything else can be typed or pasted.
const PICKER_EMOJI: &[&str] = &[
    "📋", "📝", "✅", "📌", "📎", "📁", "📂", "🗂️", "📦", "🛒", "🛍️", "🎁", "🏠", "🏢", "🚗", "✈️",
    "🧳", "🏖️", "⛺", "🌳", "🌱", "🌸", "☀️", "🌙", "⭐", "🔥", "💡", "🚀", "🎯", "🏆", "💰", "💳",
    "📚", "🎓", "💻", "🛠️", "🔧", "🎨", "🎵", "🎬", "🎮", "🏋️", "🚴", "🍎", "🍕", "☕", "🍺", "🎂",
    "💊", "🩺", "👶", "🐶", "🐱", "❤️", "😀", "🧠", "📅", "⏰", "🧹", "🧺",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerTab {
    Icons,
    Emoji,
}

/// Tabbed picker for an entity icon: lucide icons (with search), emoji, or the
/// per-entity default.
///
/// `value` is the stored icon string (`lucide:<name>`, `emoji:<glyph>`, a legacy
/// bare emoji or `None`); `on_change` receives the new stored string, or `None`
/// for "back to the default". The picker owns no state besides its tab, search
/// text and emoji input, so a dialog or popover can embed it.
#[component]
pub fn IconPicker(
    #[prop(into)] value: Signal<Option<String>>,
    on_change: Callback<Option<String>>,
    /// Lucide name shown on the "default icon" button.
    default_icon: &'static str,
) -> impl IntoView {
    let tab = RwSignal::new(PickerTab::Icons);
    let query = RwSignal::new(String::new());
    let custom_emoji = RwSignal::new(String::new());

    let selected_lucide = move || match parse_icon_value(value.get().as_deref()) {
        IconValue::Lucide(name) => Some(name.to_string()),
        _ => None,
    };
    let selected_emoji = move || match parse_icon_value(value.get().as_deref()) {
        IconValue::Emoji(emoji) => Some(emoji.to_string()),
        _ => None,
    };
    let is_default = move || matches!(parse_icon_value(value.get().as_deref()), IconValue::None);

    let tab_button = move |kind: PickerTab, testid: &'static str, label: Signal<String>| {
        view! {
            <button
                type="button"
                role="tab"
                class=move || if tab.get() == kind { "tab tab-active" } else { "tab" }
                aria-selected=move || (tab.get() == kind).to_string()
                data-testid=testid
                on:click=move |_| tab.set(kind)
            >
                {move || label.get()}
            </button>
        }
    };

    let use_custom_emoji = move || {
        let emoji = custom_emoji.get();
        let emoji = emoji.trim();
        if !emoji.is_empty() {
            on_change.run(Some(format!("emoji:{emoji}")));
            custom_emoji.set(String::new());
        }
    };

    view! {
        <div class="flex flex-col gap-2" data-testid="icon-picker">
            <div class="flex items-center gap-2">
                <div role="tablist" class="tabs tabs-box tabs-sm">
                    {tab_button(PickerTab::Icons, "icon-picker-tab-icons", move_tr!("appearance-tab-icons"))}
                    {tab_button(PickerTab::Emoji, "icon-picker-tab-emoji", move_tr!("appearance-tab-emoji"))}
                </div>
                <button
                    type="button"
                    class=move || if is_default() { "btn btn-sm btn-primary btn-soft ml-auto gap-1" } else { "btn btn-sm btn-ghost ml-auto gap-1" }
                    aria-pressed=move || is_default().to_string()
                    data-testid="icon-picker-default"
                    on:click=move |_| on_change.run(None)
                >
                    <Icon name=default_icon size=16 />
                    {move_tr!("appearance-icon-default")}
                </button>
            </div>

            {move || match tab.get() {
                PickerTab::Icons => view! {
                    <div role="tabpanel" class="flex flex-col gap-2">
                        <input
                            type="search"
                            class="input input-sm input-bordered w-full"
                            placeholder=move_tr!("appearance-search")
                            aria-label=move_tr!("appearance-search")
                            data-testid="icon-picker-search"
                            prop:value=move || query.get()
                            on:input=move |ev| query.set(event_target_value(&ev))
                        />
                        // The grid is the only part that follows the query, so typing never
                        // rebuilds (and un-focuses) the search input.
                        {move || {
                            let needle = query.get().trim().to_lowercase();
                            let matches: Vec<&'static str> = PICKER_ICONS
                                .iter()
                                .copied()
                                .filter(|name| icon_exists(name) && name.contains(needle.as_str()))
                                .collect();
                            view! {
                                {matches.is_empty().then(|| view! {
                                    <p class="text-sm text-base-content/60 py-2">{move_tr!("appearance-search-empty")}</p>
                                })}
                                <div class="grid grid-cols-8 gap-1 max-h-48 overflow-y-auto p-1">
                                    {matches
                                        .into_iter()
                                        .map(|name| {
                                            view! {
                                                <button
                                                    type="button"
                                                    class=move || {
                                                        if selected_lucide().as_deref() == Some(name) {
                                                            "btn btn-square btn-sm btn-primary btn-soft"
                                                        } else {
                                                            "btn btn-square btn-sm btn-ghost"
                                                        }
                                                    }
                                                    title=name
                                                    aria-label=name
                                                    aria-pressed=move || (selected_lucide().as_deref() == Some(name)).to_string()
                                                    data-testid=format!("icon-picker-icon-{name}")
                                                    on:click=move |_| on_change.run(Some(format!("lucide:{name}")))
                                                >
                                                    <Icon name=name size=18 />
                                                </button>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                        }}
                    </div>
                }
                .into_any(),
                PickerTab::Emoji => view! {
                    <div role="tabpanel" class="flex flex-col gap-2">
                        <div class="flex gap-2">
                            <input
                                type="text"
                                class="input input-sm input-bordered flex-1"
                                placeholder=move_tr!("appearance-emoji-custom")
                                aria-label=move_tr!("appearance-emoji-custom")
                                data-testid="icon-picker-emoji-input"
                                prop:value=move || custom_emoji.get()
                                on:input=move |ev| custom_emoji.set(event_target_value(&ev))
                                on:keydown=move |ev: KeyboardEvent| {
                                    if ev.key() == "Enter" {
                                        ev.prevent_default();
                                        use_custom_emoji();
                                    }
                                }
                            />
                            <button
                                type="button"
                                class="btn btn-sm btn-primary"
                                data-testid="icon-picker-emoji-use"
                                on:click=move |_| use_custom_emoji()
                            >
                                {move_tr!("appearance-emoji-use")}
                            </button>
                        </div>
                        <div class="grid grid-cols-8 gap-1 max-h-48 overflow-y-auto p-1">
                            {PICKER_EMOJI
                                .iter()
                                .copied()
                                .map(|emoji| {
                                    view! {
                                        <button
                                            type="button"
                                            class=move || {
                                                if selected_emoji().as_deref() == Some(emoji) {
                                                    "btn btn-square btn-sm btn-primary btn-soft text-lg"
                                                } else {
                                                    "btn btn-square btn-sm btn-ghost text-lg"
                                                }
                                            }
                                            aria-label=emoji
                                            aria-pressed=move || (selected_emoji().as_deref() == Some(emoji)).to_string()
                                            data-testid="icon-picker-emoji"
                                            on:click=move |_| on_change.run(Some(format!("emoji:{emoji}")))
                                        >
                                            {emoji}
                                        </button>
                                    }
                                })
                                .collect_view()}
                        </div>
                    </div>
                }
                .into_any(),
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_picker_icon_renders() {
        for name in PICKER_ICONS {
            assert!(icon_exists(name), "{name} is not in the icon table");
        }
    }

    #[test]
    fn picker_icons_are_unique() {
        let mut names: Vec<&str> = PICKER_ICONS.to_vec();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
    }
}
