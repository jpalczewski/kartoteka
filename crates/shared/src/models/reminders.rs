use serde::{Deserialize, Serialize};

/// Upper bound for the number of reminder offsets per date field.
const MAX_REMINDERS_PER_FIELD: usize = 5;
/// Upper bound for a single reminder offset: 28 days expressed in minutes.
const MAX_REMINDER_MINUTES: u32 = 40_320;

/// User default reminder offsets, stored as JSON in `user_settings` under
/// [`REMINDER_DEFAULTS_KEY`](crate::constants::REMINDER_DEFAULTS_KEY).
///
/// Every value is a number of minutes **before** the event date it belongs to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReminderDefaults {
    #[serde(default)]
    pub start_date: Vec<u32>,
    #[serde(default)]
    pub deadline: Vec<u32>,
    #[serde(default)]
    pub hard_deadline: Vec<u32>,
}

impl Default for ReminderDefaults {
    fn default() -> Self {
        Self {
            start_date: Vec::new(),
            deadline: vec![60],
            hard_deadline: vec![1440, 60],
        }
    }
}

impl ReminderDefaults {
    /// Checks the stored defaults: at most 5 offsets per field, each at most 28 days.
    pub fn validate(&self) -> Result<(), &'static str> {
        let fields = [&self.start_date, &self.deadline, &self.hard_deadline];
        if fields
            .iter()
            .any(|offsets| offsets.len() > MAX_REMINDERS_PER_FIELD)
        {
            return Err("at most 5 reminders per date field");
        }
        if fields
            .iter()
            .any(|offsets| offsets.iter().any(|m| *m > MAX_REMINDER_MINUTES))
        {
            return Err("reminder offset must be at most 40320 minutes (28 days)");
        }
        Ok(())
    }
}
