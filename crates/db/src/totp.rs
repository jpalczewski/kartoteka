use crate::{DbError, SqlitePool};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TotpRow {
    pub user_id: String,
    pub secret: String,
    pub verified: bool,
    pub created_at: String,
}

/// Insert or replace a TOTP secret for the user. Sets verified = 0.
pub async fn upsert(pool: &SqlitePool, user_id: &str, secret: &str) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO totp_secrets (user_id, secret, verified) VALUES (?, ?, 0)
         ON CONFLICT(user_id) DO UPDATE SET secret = excluded.secret, verified = 0",
    )
    .bind(user_id)
    .bind(secret)
    .execute(pool)
    .await?;
    Ok(())
}

/// Find the TOTP row for a user, if any.
pub async fn find(pool: &SqlitePool, user_id: &str) -> Result<Option<TotpRow>, DbError> {
    Ok(sqlx::query_as::<_, TotpRow>(
        "SELECT user_id, secret, verified, created_at FROM totp_secrets WHERE user_id = ?",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?)
}

/// Mark the TOTP secret as verified (setup complete).
pub async fn mark_verified(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    sqlx::query("UPDATE totp_secrets SET verified = 1 WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Delete the TOTP secret (disables 2FA).
pub async fn delete(pool: &SqlitePool, user_id: &str) -> Result<(), DbError> {
    sqlx::query("DELETE FROM totp_secrets WHERE user_id = ?")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Returns true iff the user has a verified TOTP secret.
pub async fn is_enabled(pool: &SqlitePool, user_id: &str) -> Result<bool, DbError> {
    Ok(find(pool, user_id)
        .await?
        .map(|r| r.verified)
        .unwrap_or(false))
}

/// Atomically record that a TOTP code was accepted for a user. Returns true
/// if the insert succeeded (code not yet used), false on PRIMARY KEY conflict
/// (replay attempt). Also GCs rows older than 5 minutes — well beyond the
/// ~90s validity window of any code under skew=1/step=30s.
pub async fn try_mark_code_used(
    pool: &SqlitePool,
    user_id: &str,
    code: &str,
) -> Result<bool, DbError> {
    // Best-effort cleanup of stale rows; ignore failures.
    let _ =
        sqlx::query("DELETE FROM totp_used_codes WHERE used_at < datetime('now', '-5 minutes')")
            .execute(pool)
            .await;

    let result = sqlx::query("INSERT OR IGNORE INTO totp_used_codes (user_id, code) VALUES (?, ?)")
        .bind(user_id)
        .bind(code)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_pool;

    #[tokio::test]
    async fn insert_and_find_totp_secret() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "JBSWY3DPEHPK3PXP").await.unwrap();
        let row = find(&pool, "u1").await.unwrap().unwrap();
        assert_eq!(row.secret, "JBSWY3DPEHPK3PXP");
        assert!(!row.verified);
    }

    #[tokio::test]
    async fn upsert_replaces_existing_secret() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "OLDSECRET").await.unwrap();
        mark_verified(&pool, "u1").await.unwrap();
        upsert(&pool, "u1", "NEWSECRET").await.unwrap();
        let row = find(&pool, "u1").await.unwrap().unwrap();
        assert_eq!(row.secret, "NEWSECRET");
        assert!(!row.verified); // reset to unverified
    }

    #[tokio::test]
    async fn mark_verified_updates_flag() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "SECRET").await.unwrap();
        mark_verified(&pool, "u1").await.unwrap();
        let row = find(&pool, "u1").await.unwrap().unwrap();
        assert!(row.verified);
    }

    #[tokio::test]
    async fn delete_removes_secret() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "SECRET").await.unwrap();
        delete(&pool, "u1").await.unwrap();
        assert!(find(&pool, "u1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn is_enabled_returns_false_when_unverified() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "SECRET").await.unwrap();
        assert!(!is_enabled(&pool, "u1").await.unwrap());
    }

    #[tokio::test]
    async fn is_enabled_returns_true_when_verified() {
        let pool = test_pool().await;
        sqlx::query("INSERT INTO users (id, email) VALUES ('u1', 'a@b.com')")
            .execute(&pool)
            .await
            .unwrap();
        upsert(&pool, "u1", "SECRET").await.unwrap();
        mark_verified(&pool, "u1").await.unwrap();
        assert!(is_enabled(&pool, "u1").await.unwrap());
    }
}
