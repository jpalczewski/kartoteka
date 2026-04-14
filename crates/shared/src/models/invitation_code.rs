use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationCode {
    pub id: String,
    pub code: String,
    pub created_by: String,
    pub used_by: Option<String>,
    pub reserved_by_email: Option<String>,
    pub reserved_until: Option<String>,
    pub created_at: String,
    pub used_at: Option<String>,
    pub expires_at: Option<String>,
}
