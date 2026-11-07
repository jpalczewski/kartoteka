//! Reusable UI primitives (icons, icon buttons, chips, menus, …).
//!
//! These components are presentation-only: they own no server state and no
//! business rules, so pages and feature components can share one look without
//! duplicating markup.

pub mod action_menu;
pub mod chip;
pub mod completion_bar;
pub mod icon;
pub mod icon_button;
pub mod section_header;

pub use action_menu::{ActionMenu, ActionMenuItem};
pub use chip::{Chip, ChipTone};
pub use completion_bar::CompletionBar;
pub use icon::{EntityIcon, Icon, IconName};
pub use icon_button::{IconButton, IconButtonSize, IconButtonVariant};
pub use section_header::SectionHeader;
