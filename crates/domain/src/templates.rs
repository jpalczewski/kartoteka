use crate::{DomainError, lists::row_to_list};
use kartoteka_db as db;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

// ── Public domain types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateItem {
    pub id: String,
    pub template_id: String,
    pub title: String,
    pub description: Option<String>,
    pub position: i32,
    pub quantity: Option<i32>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateWithItems {
    pub template: Template,
    pub items: Vec<TemplateItem>,
    pub tag_ids: Vec<String>,
}

// ── Conversion helpers ────────────────────────────────────────────────────────

fn row_to_template(row: db::types::TemplateRow) -> Template {
    Template {
        id: row.id,
        user_id: row.user_id,
        name: row.name,
        icon: row.icon,
        description: row.description,
        created_at: row.created_at,
    }
}

fn row_to_item(row: db::types::TemplateItemRow) -> TemplateItem {
    TemplateItem {
        id: row.id,
        template_id: row.template_id,
        title: row.title,
        description: row.description,
        position: row.position,
        quantity: row.quantity,
        unit: row.unit,
    }
}

async fn build_with_items(
    pool: &SqlitePool,
    template: Template,
) -> Result<TemplateWithItems, DomainError> {
    let item_rows = db::templates::get_items(pool, &template.id).await?;
    let tag_rows = db::templates::get_tags(pool, &template.id).await?;
    let items = item_rows.into_iter().map(row_to_item).collect();
    let tag_ids = tag_rows.into_iter().map(|t| t.id).collect();
    Ok(TemplateWithItems {
        template,
        items,
        tag_ids,
    })
}

// ── Orchestration ─────────────────────────────────────────────────────────────

#[tracing::instrument(skip(pool))]
pub async fn list(pool: &SqlitePool, user_id: &str) -> Result<Vec<TemplateWithItems>, DomainError> {
    let rows = db::templates::list_all(pool, user_id).await?;
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let template = row_to_template(row);
        result.push(build_with_items(pool, template).await?);
    }
    Ok(result)
}

#[tracing::instrument(skip(pool))]
pub async fn get(
    pool: &SqlitePool,
    user_id: &str,
    id: &str,
) -> Result<Option<TemplateWithItems>, DomainError> {
    match db::templates::get_one(pool, id, user_id).await? {
        None => Ok(None),
        Some(row) => {
            let template = row_to_template(row);
            Ok(Some(build_with_items(pool, template).await?))
        }
    }
}

#[tracing::instrument(skip(pool))]
pub async fn create_from_list(
    pool: &SqlitePool,
    user_id: &str,
    list_id: &str,
    template_name: &str,
) -> Result<TemplateWithItems, DomainError> {
    // Phase 1: READ
    let _list = db::lists::get_one(pool, list_id, user_id)
        .await?
        .ok_or(DomainError::Forbidden)?;

    let item_rows = db::items::list_for_list(pool, list_id, user_id).await?;
    let tag_rows = db::tags::get_tags_for_list(pool, list_id, user_id).await?;

    // Phase 3: WRITE
    let template_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;

    db::templates::insert(&mut tx, &template_id, user_id, template_name, None, None).await?;

    for item in &item_rows {
        let item_id = Uuid::new_v4().to_string();
        db::templates::insert_item(
            &mut tx,
            &item_id,
            &template_id,
            &item.title,
            item.description.as_deref(),
            item.position,
            item.quantity,
            item.unit.as_deref(),
        )
        .await?;
    }

    for tag in &tag_rows {
        db::templates::insert_tag(&mut tx, &template_id, &tag.id).await?;
    }

    tx.commit().await.map_err(db::DbError::Sqlx)?;

    get(pool, user_id, &template_id)
        .await?
        .ok_or_else(|| DomainError::Internal("template disappeared after create".into()))
}

#[tracing::instrument(skip(pool))]
pub async fn create_list_from_template(
    pool: &SqlitePool,
    user_id: &str,
    template_id: &str,
    list_name: &str,
    list_type: &str,
) -> Result<crate::lists::List, DomainError> {
    // Phase 1: READ
    let tmpl = get(pool, user_id, template_id)
        .await?
        .ok_or(DomainError::NotFound("template"))?;

    let position = db::lists::next_position(pool, user_id, None, None).await?;

    // Phase 3: WRITE
    let list_id = Uuid::new_v4().to_string();
    let mut tx = pool.begin().await.map_err(db::DbError::Sqlx)?;

    db::lists::insert(
        &mut tx, &list_id, user_id, position, list_name, None, None, list_type, None, None,
    )
    .await?;

    for item in &tmpl.items {
        let item_id = Uuid::new_v4().to_string();
        db::items::insert_in_tx(
            &mut tx,
            &db::items::InsertItemInput {
                id: item_id,
                list_id: list_id.clone(),
                position: item.position,
                title: item.title.clone(),
                description: item.description.clone(),
                quantity: item.quantity,
                unit: item.unit.clone(),
                ..Default::default()
            },
        )
        .await?;
    }

    tx.commit().await.map_err(db::DbError::Sqlx)?;

    // Add tags after commit (uses pool, not tx)
    for tag_id in &tmpl.tag_ids {
        db::tags::add_list_tag(pool, &list_id, tag_id, user_id).await?;
    }

    db::lists::get_one(pool, &list_id, user_id)
        .await?
        .map(row_to_list)
        .transpose()?
        .ok_or_else(|| {
            DomainError::Internal("list disappeared after create_list_from_template".into())
        })
}

#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &SqlitePool, user_id: &str, id: &str) -> Result<bool, DomainError> {
    Ok(db::templates::delete(pool, id, user_id).await?)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use kartoteka_db::test_helpers::{create_test_user, test_pool};

    async fn make_list(pool: &SqlitePool, uid: &str, name: &str) -> crate::lists::List {
        crate::lists::create(
            pool,
            uid,
            &crate::lists::CreateListRequest {
                name: name.into(),
                list_type: None,
                icon: None,
                description: None,
                container_id: None,
                parent_list_id: None,
                features: vec![],
            },
        )
        .await
        .unwrap()
    }

    async fn add_item(pool: &SqlitePool, uid: &str, list_id: &str, title: &str) {
        crate::items::create(
            pool,
            uid,
            list_id,
            &crate::items::CreateItemRequest {
                title: title.into(),
                description: None,
                quantity: None,
                actual_quantity: None,
                unit: None,
                start_date: None,
                start_time: None,
                deadline: None,
                deadline_time: None,
                hard_deadline: None,
                estimated_duration: None,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn list_returns_templates_for_user_only() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;

        let l1 = make_list(&pool, &uid, "ListA").await;
        let l2 = make_list(&pool, &uid2, "ListB").await;

        create_from_list(&pool, &uid, &l1.id, "TemplateA")
            .await
            .unwrap();
        create_from_list(&pool, &uid2, &l2.id, "TemplateB")
            .await
            .unwrap();

        let templates = list(&pool, &uid).await.unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].template.name, "TemplateA");
    }

    #[tokio::test]
    async fn create_from_list_snapshots_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = make_list(&pool, &uid, "MyList").await;

        add_item(&pool, &uid, &list.id, "Item One").await;
        add_item(&pool, &uid, &list.id, "Item Two").await;

        let tmpl = create_from_list(&pool, &uid, &list.id, "Snapshot")
            .await
            .unwrap();

        assert_eq!(tmpl.template.name, "Snapshot");
        assert_eq!(tmpl.items.len(), 2);
        let titles: Vec<&str> = tmpl.items.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&"Item One"));
        assert!(titles.contains(&"Item Two"));
    }

    #[tokio::test]
    async fn create_list_from_template_creates_list_with_items() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = make_list(&pool, &uid, "Source").await;

        add_item(&pool, &uid, &list.id, "Alpha").await;
        add_item(&pool, &uid, &list.id, "Beta").await;

        let tmpl = create_from_list(&pool, &uid, &list.id, "T1").await.unwrap();

        let new_list =
            create_list_from_template(&pool, &uid, &tmpl.template.id, "NewList", "checklist")
                .await
                .unwrap();

        assert_eq!(new_list.name, "NewList");

        let items = crate::items::list_for_list(&pool, &new_list.id, &uid)
            .await
            .unwrap();
        assert_eq!(items.len(), 2);
        let titles: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&"Alpha"));
        assert!(titles.contains(&"Beta"));
    }

    #[tokio::test]
    async fn delete_removes_template() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let list = make_list(&pool, &uid, "L").await;

        let tmpl = create_from_list(&pool, &uid, &list.id, "ToDelete")
            .await
            .unwrap();

        let deleted = delete(&pool, &uid, &tmpl.template.id).await.unwrap();
        assert!(deleted);

        let found = get(&pool, &uid, &tmpl.template.id).await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn create_from_list_wrong_user_returns_forbidden() {
        let pool = test_pool().await;
        let uid = create_test_user(&pool).await;
        let uid2 = create_test_user(&pool).await;
        let list = make_list(&pool, &uid, "Private").await;

        let err = create_from_list(&pool, &uid2, &list.id, "ShouldFail")
            .await
            .unwrap_err();
        assert!(matches!(err, DomainError::Forbidden));
    }
}
