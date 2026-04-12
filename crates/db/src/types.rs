use kartoteka_shared::types::FlexDate;

// --- sqlx row types ---

#[derive(Debug, sqlx::FromRow)]
pub struct UserRow {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct AuthMethodRow {
    pub id: String,
    pub user_id: String,
    pub provider: String,
    pub provider_id: String,
    pub credential: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TotpSecretRow {
    pub user_id: String,
    pub secret: String,
    pub verified: bool,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PersonalTokenRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scope: String,
    pub last_used_at: Option<String>,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ServerConfigRow {
    pub key: String,
    pub value: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OAuthClientRow {
    pub client_id: String,
    pub client_name: Option<String>,
    pub redirect_uris: String,
    pub grant_types: String,
    pub token_endpoint_auth_method: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OAuthAuthCodeRow {
    pub code: String,
    pub client_id: String,
    pub user_id: String,
    pub redirect_uri: String,
    pub code_challenge: String,
    pub code_challenge_method: String,
    pub scopes: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct OAuthRefreshTokenRow {
    pub token: String,
    pub client_id: String,
    pub user_id: String,
    pub scopes: Option<String>,
    pub expires_at: String,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ContainerRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub parent_container_id: Option<String>,
    pub position: i32,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ListRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub list_type: String,
    pub parent_list_id: Option<String>,
    pub position: i32,
    pub archived: bool,
    pub container_id: Option<String>,
    pub pinned: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ItemRow {
    pub id: String,
    pub list_id: String,
    pub title: String,
    pub description: Option<String>,
    pub completed: bool,
    pub position: i32,
    pub quantity: Option<i32>,
    pub actual_quantity: Option<i32>,
    pub unit: Option<String>,
    pub start_date: Option<FlexDate>,
    pub start_time: Option<String>,
    pub deadline: Option<FlexDate>,
    pub deadline_time: Option<String>,
    pub hard_deadline: Option<FlexDate>,
    pub estimated_duration: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TagRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_tag_id: Option<String>,
    pub tag_type: String,
    pub metadata: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ItemTagRow {
    pub item_id: String,
    pub tag_id: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ListTagRow {
    pub list_id: String,
    pub tag_id: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ContainerTagRow {
    pub container_id: String,
    pub tag_id: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ListFeatureRow {
    pub list_id: String,
    pub feature_name: String,
    pub config: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct UserSettingRow {
    pub user_id: String,
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct CommentRow {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub content: String,
    pub author_type: String,
    pub author_name: Option<String>,
    pub user_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct EntityRelationRow {
    pub id: String,
    pub from_type: String,
    pub from_id: String,
    pub to_type: String,
    pub to_id: String,
    pub relation_type: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TimeEntryRow {
    pub id: String,
    pub item_id: Option<String>,
    pub user_id: String,
    pub description: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub duration: Option<i32>,
    pub source: String,
    pub mode: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateRow {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateItemRow {
    pub id: String,
    pub template_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
    pub created_at: String,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TemplateTagRow {
    pub template_id: String,
    pub tag_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;
    use kartoteka_shared::types::FlexDate;

    #[tokio::test]
    async fn flex_date_roundtrip_day() {
        let pool = test_pool().await;
        let date = FlexDate::Day(chrono::NaiveDate::from_ymd_opt(2026, 5, 15).unwrap());

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, start_date) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05-15");

        let decoded: (FlexDate,) = sqlx::query_as("SELECT start_date FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(decoded.0, date);
    }

    #[tokio::test]
    async fn flex_date_roundtrip_week() {
        let pool = test_pool().await;
        let date = FlexDate::Week(2026, 20);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-W20");
    }

    #[tokio::test]
    async fn flex_date_roundtrip_month() {
        let pool = test_pool().await;
        let date = FlexDate::Month(2026, 5);

        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO lists (id, user_id, name) VALUES ('l1', 'u1', 'test')")
            .execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO items (id, list_id, title, hard_deadline) VALUES ('i1', 'l1', 'test', ?)")
            .bind(&date)
            .execute(&pool).await.unwrap();

        let row: (String,) = sqlx::query_as("SELECT hard_deadline FROM items WHERE id = 'i1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(row.0, "2026-05");
    }

    #[tokio::test]
    async fn user_row_from_db() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email, name, role) VALUES ('u1', 'a@b.com', 'Alice', 'admin')")
            .execute(&pool).await.unwrap();

        let user: UserRow = sqlx::query_as("SELECT id, email, name, avatar_url, role, created_at, updated_at FROM users WHERE id = 'u1'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(user.id, "u1");
        assert_eq!(user.email, "a@b.com");
        assert_eq!(user.name.as_deref(), Some("Alice"));
        assert_eq!(user.role, "admin");
    }
}
