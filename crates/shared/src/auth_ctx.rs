use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserLocale(pub String);
