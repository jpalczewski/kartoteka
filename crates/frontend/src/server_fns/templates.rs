#[allow(unused_imports)]
use kartoteka_shared::types::{TemplateItem, TemplateWithItems};
use leptos::prelude::*;

#[cfg(feature = "ssr")]
use {
    crate::server_fns::utils::{SfnResultExt, require_user},
    kartoteka_domain as domain,
};

#[cfg(feature = "ssr")]
fn domain_tmpl_to_shared(t: domain::templates::TemplateWithItems) -> TemplateWithItems {
    TemplateWithItems {
        id: t.template.id,
        user_id: t.template.user_id,
        name: t.template.name,
        icon: t.template.icon,
        description: t.template.description,
        items: t
            .items
            .into_iter()
            .map(|i| TemplateItem {
                id: i.id,
                template_id: i.template_id,
                title: i.title,
                description: i.description,
                position: i.position,
                quantity: i.quantity,
                unit: i.unit,
            })
            .collect(),
        tag_ids: t.tag_ids,
        created_at: t.template.created_at,
    }
}

#[server(prefix = "/leptos")]
pub async fn get_templates() -> Result<Vec<TemplateWithItems>, ServerFnError> {
    let (pool, user) = require_user().await?;

    let templates = domain::templates::list(&pool, &user.id).await.sfn()?;

    Ok(templates.into_iter().map(domain_tmpl_to_shared).collect())
}

#[server(prefix = "/leptos")]
pub async fn create_template_from_list(
    list_id: String,
    name: String,
) -> Result<TemplateWithItems, ServerFnError> {
    if name.trim().is_empty() {
        return Err(ServerFnError::new("name cannot be empty".to_string()));
    }
    let (pool, user) = require_user().await?;

    let template = domain::templates::create_from_list(&pool, &user.id, &list_id, &name)
        .await
        .sfn()?;

    Ok(domain_tmpl_to_shared(template))
}

#[server(prefix = "/leptos")]
pub async fn delete_template(id: String) -> Result<(), ServerFnError> {
    let (pool, user) = require_user().await?;

    let deleted = domain::templates::delete(&pool, &user.id, &id)
        .await
        .sfn()?;

    if !deleted {
        return Err(ServerFnError::new("template not found".to_string()));
    }
    Ok(())
}
