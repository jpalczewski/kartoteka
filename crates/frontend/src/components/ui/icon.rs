use kartoteka_shared::{IconValue, parse_icon_value};
use leptos::prelude::*;
use lucide_leptos::{
    AlarmClock, Archive, ArrowLeft, ArrowUpRight, Calendar, Check, CheckCheck, ChevronDown,
    ChevronLeft, ChevronRight, ChevronUp, Circle, CircleAlert, CircleCheck, Clock, ClockAlert,
    Columns3, Copy, Ellipsis, EllipsisVertical, ExternalLink, Folder, FolderKanban, FolderOpen,
    GripVertical, Hourglass, House, Inbox, LayoutGrid, Link, List, LogOut, Luggage, MapPin,
    Package, PackageOpen, Pencil, Pin, PinOff, Play, Plus, Repeat, RotateCcw, Search, Settings,
    ShoppingCart, Shuffle, Square, SquareCheck, Star, Tag, Timer, Trash, TriangleAlert, Undo2, X,
};

/// A lucide icon name, accepted from `&'static str`, `String` or `Signal<String>`.
///
/// `#[prop(into)] name: IconName` keeps call sites terse (`name="trash"`) while
/// still allowing reactive names (`name=move || ...`).
#[derive(Clone, Debug)]
pub struct IconName(pub Signal<String>);

impl From<&'static str> for IconName {
    fn from(name: &'static str) -> Self {
        Self(Signal::derive(move || name.to_string()))
    }
}

impl From<String> for IconName {
    fn from(name: String) -> Self {
        Self(Signal::derive(move || name.clone()))
    }
}

impl From<Signal<String>> for IconName {
    fn from(name: Signal<String>) -> Self {
        Self(name)
    }
}

/// Maps one of our icon names to the lucide component that draws it.
///
/// Only the curated set below is compiled in (`lucide-leptos` is built with a
/// handful of category features, not `all-icons`); every other name renders
/// nothing so a bad stored value can never break a page.
macro_rules! match_icon {
    ($name:expr, $size:expr, $color:expr, $($key:literal => $component:ident),* $(,)?) => {
        match $name {
            $(
                $key => view! { <$component size=$size color=$color /> }.into_any(),
            )*
            _ => ().into_any(),
        }
    };
}

/// Renders the SVG of the curated lucide `name`, or nothing when it is unknown.
fn render_icon(name: &str, size: Signal<usize>, color: Signal<String>) -> AnyView {
    // Aliases: names that appear in stored data or older lucide versions.
    let name = match name {
        "trash-2" => "trash",
        "more-horizontal" => "ellipsis",
        "more-vertical" => "ellipsis-vertical",
        "home" => "house",
        other => other,
    };

    match_icon! { name, size, color,
        "alarm-clock" => AlarmClock,
        "archive" => Archive,
        "arrow-left" => ArrowLeft,
        "arrow-up-right" => ArrowUpRight,
        "calendar" => Calendar,
        "check" => Check,
        "check-check" => CheckCheck,
        "chevron-down" => ChevronDown,
        "chevron-left" => ChevronLeft,
        "chevron-right" => ChevronRight,
        "chevron-up" => ChevronUp,
        "circle" => Circle,
        "circle-alert" => CircleAlert,
        "circle-check" => CircleCheck,
        "clock" => Clock,
        "clock-alert" => ClockAlert,
        "columns-3" => Columns3,
        "copy" => Copy,
        "ellipsis" => Ellipsis,
        "ellipsis-vertical" => EllipsisVertical,
        "external-link" => ExternalLink,
        "folder" => Folder,
        "folder-open" => FolderOpen,
        "folder-kanban" => FolderKanban,
        "grip-vertical" => GripVertical,
        "house" => House,
        "hourglass" => Hourglass,
        "inbox" => Inbox,
        "layout-grid" => LayoutGrid,
        "link" => Link,
        "list" => List,
        "log-out" => LogOut,
        "luggage" => Luggage,
        "map-pin" => MapPin,
        "pencil" => Pencil,
        "pin" => Pin,
        "package" => Package,
        "package-open" => PackageOpen,
        "pin-off" => PinOff,
        "play" => Play,
        "plus" => Plus,
        "repeat" => Repeat,
        "rotate-ccw" => RotateCcw,
        "search" => Search,
        "settings" => Settings,
        "shopping-cart" => ShoppingCart,
        "shuffle" => Shuffle,
        "square" => Square,
        "square-check" => SquareCheck,
        "star" => Star,
        "tag" => Tag,
        "timer" => Timer,
        "trash" => Trash,
        "triangle-alert" => TriangleAlert,
        "undo-2" => Undo2,
        "x" => X,
    }
}

/// A single SVG icon from the curated lucide set.
///
/// `name` is a lucide name (`"trash"`, `"square-check"`); unknown names render
/// nothing. `class` is applied to the wrapping `<span>`, which is what makes an
/// icon inherit sizing/colour utilities from its parent button or chip.
#[component]
pub fn Icon(
    #[prop(into)] name: IconName,
    /// Pixel size of the square icon.
    #[prop(default = 18)]
    size: usize,
    /// Extra classes for the wrapping span.
    #[prop(default = String::new(), into)]
    class: String,
    /// CSS colour for the stroke; defaults to `currentColor`.
    #[prop(optional_no_strip)]
    color: Option<String>,
) -> impl IntoView {
    let IconName(name) = name;
    let color = color.unwrap_or_else(|| "currentColor".to_string());
    let size_signal = Signal::from(size);
    let color_signal = Signal::from(color);

    view! {
        <span class=format!("inline-flex shrink-0 items-center justify-center {class}")>
            {move || render_icon(&name.get(), size_signal, color_signal)}
        </span>
    }
}

/// An entity's stored icon: a lucide icon, an emoji, or a fallback lucide icon.
///
/// `raw` is the value as stored in the database (`lucide:<name>`,
/// `emoji:<glyph>` or a legacy bare emoji) and is parsed with
/// [`parse_icon_value`]; anything unusable falls back to `fallback`.
#[component]
pub fn EntityIcon(
    /// Raw stored icon value.
    #[prop(optional_no_strip)]
    raw: Option<String>,
    /// Lucide name used when `raw` is absent or malformed.
    #[prop(into)]
    fallback: IconName,
    /// CSS colour applied to the lucide stroke or the emoji text.
    #[prop(optional_no_strip)]
    color: Option<String>,
    /// Pixel size of the square icon.
    #[prop(default = 18)]
    size: usize,
    /// Extra classes for the wrapping element.
    #[prop(default = String::new(), into)]
    class: String,
) -> impl IntoView {
    let IconName(fallback) = fallback;

    view! {
        {move || match parse_icon_value(raw.as_deref()) {
            IconValue::Lucide(name) => view! {
                <Icon name=name.to_string() size=size class=class.clone() color=color.clone() />
            }
            .into_any(),
            IconValue::Emoji(emoji) => view! {
                <span
                    class=format!("inline-flex shrink-0 items-center justify-center {class}")
                    style=color.as_ref().map(|c| format!("color: {c};"))
                >
                    {emoji.to_string()}
                </span>
            }
            .into_any(),
            IconValue::None => view! {
                <Icon name=fallback size=size class=class.clone() color=color.clone() />
            }
            .into_any(),
        }}
    }
}
