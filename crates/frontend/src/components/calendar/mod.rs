pub mod calendar_nav;
pub mod month_grid;
pub mod week_view;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewMode {
    Month,
    Week,
}
