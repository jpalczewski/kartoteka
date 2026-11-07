//! Reusable UI primitives (icons, icon buttons, chips, menus, …).
//!
//! These components are presentation-only: they own no server state and no
//! business rules, so pages and feature components can share one look without
//! duplicating markup.

pub mod action_menu;
pub mod appearance_dialog;
pub mod chip;
pub mod completion_bar;
pub mod icon;
pub mod icon_button;
pub mod icon_picker;
pub mod section_header;

pub use action_menu::{ActionMenu, ActionMenuItem};
pub use appearance_dialog::AppearanceDialog;
pub use chip::{Chip, ChipTone};
pub use completion_bar::CompletionBar;
pub use icon::{EntityIcon, Icon, IconName, icon_exists};
pub use icon_button::{IconButton, IconButtonSize, IconButtonVariant};
pub use icon_picker::IconPicker;
pub use section_header::SectionHeader;
