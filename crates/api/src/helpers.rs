#![allow(dead_code)]

use worker::*;

pub async fn verify_list_belongs_to_user(
    d1: &D1Database,
    list_id: &str,
    user_id: &str,
) -> Result<bool> {
    let check = d1
        .prepare("SELECT id FROM lists WHERE id = ?1 AND user_id = ?2")
        .bind(&[list_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.is_some())
}

pub async fn verify_item_belongs_to_user(
    d1: &D1Database,
    item_id: &str,
    user_id: &str,
) -> Result<bool> {
    let check = d1
        .prepare(
            "SELECT items.id FROM items JOIN lists ON lists.id = items.list_id \
             WHERE items.id = ?1 AND lists.user_id = ?2",
        )
        .bind(&[item_id.into(), user_id.into()])?
        .first::<serde_json::Value>(None)
        .await?;
    Ok(check.is_some())
}
