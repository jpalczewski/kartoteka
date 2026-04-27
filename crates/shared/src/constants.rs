// Session keys for the 2-step TOTP login flow (shared by REST handlers and Leptos server fns).
pub const SESSION_PENDING_USER_KEY: &str = "pending_user_id";
pub const SESSION_PENDING_2FA_ATTEMPTS_KEY: &str = "pending_2fa_attempts";
pub const SESSION_MAX_2FA_ATTEMPTS: u32 = 5;
pub const SESSION_RETURN_TO_KEY: &str = "return_to";

pub const FEATURE_QUANTITY: &str = "quantity";
pub const FEATURE_DEADLINES: &str = "deadlines";
pub const FEATURE_LOCATION: &str = "location";
pub const FEATURE_CHECKLIST: &str = "checklist";
pub const FEATURE_TIME_TRACKING: &str = "time_tracking";

pub const DATE_TYPE_START: &str = "start";
pub const DATE_TYPE_DEADLINE: &str = "deadline";
pub const DATE_TYPE_HARD_DEADLINE: &str = "hard_deadline";

pub const SETTING_MCP_AUTO_ENABLE_FEATURES: &str = "mcp_auto_enable_features";

pub const INSTANCE_SETTING_REGISTRATION_MODE: &str = "registration_mode";
