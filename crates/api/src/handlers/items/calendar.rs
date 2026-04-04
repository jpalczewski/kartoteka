use crate::error::validation_error;
use kartoteka_shared::*;
use tracing::instrument;
use worker::*;

use super::{DATE_ITEM_COLS, validation::validation_field};

#[derive(Clone, Copy, Debug)]
pub(super) enum DateFieldSelector {
    All,
    One(DateField),
}

fn parse_date_field_selector(date_field: &str) -> std::result::Result<DateFieldSelector, Response> {
    match date_field {
        "all" => Ok(DateFieldSelector::All),
        "start_date" => Ok(DateFieldSelector::One(DateField::StartDate)),
        "deadline" => Ok(DateFieldSelector::One(DateField::Deadline)),
        "hard_deadline" => Ok(DateFieldSelector::One(DateField::HardDeadline)),
        _ => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field("date_field", "invalid_date_field")],
        )
        .expect("build 422 response")),
    }
}

fn query_param_with_alias(url: &Url, primary: &str, alias: Option<&str>) -> Option<String> {
    if let Some((_, value)) = url.query_pairs().find(|(k, _)| k == primary) {
        return Some(value.to_string());
    }

    let alias_key = alias?;
    url.query_pairs()
        .find(|(k, _)| k == alias_key)
        .map(|(_, value)| value.to_string())
}

fn parse_required_query_date(
    field: &str,
    value: Option<String>,
) -> std::result::Result<chrono::NaiveDate, Response> {
    let Some(value) = value else {
        return Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "required")],
        )
        .expect("build 422 response"));
    };

    match validate_business_date(&value) {
        Ok(date) => Ok(date),
        Err(DateValidationError::Invalid) => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "invalid_date")],
        )
        .expect("build 422 response")),
        Err(DateValidationError::OutOfRange) => Err(validation_error(
            "Invalid query parameters.",
            vec![validation_field(field, "date_out_of_range")],
        )
        .expect("build 422 response")),
    }
}

fn relevant_date_for_item(item: &DateItem, selector: DateFieldSelector) -> Option<&str> {
    match selector {
        DateFieldSelector::All => match item.date_type.as_deref() {
            Some("start_date") => item.start_date.as_deref(),
            Some("hard_deadline") => item.hard_deadline.as_deref(),
            Some("deadline") => item.deadline.as_deref(),
            _ => None,
        },
        DateFieldSelector::One(DateField::StartDate) => item.start_date.as_deref(),
        DateFieldSelector::One(DateField::Deadline) => item.deadline.as_deref(),
        DateFieldSelector::One(DateField::HardDeadline) => item.hard_deadline.as_deref(),
    }
}

fn keep_item_for_day(
    item: &DateItem,
    selector: DateFieldSelector,
    target: chrono::NaiveDate,
    include_overdue: bool,
) -> bool {
    let Some(date_value) = relevant_date_for_item(item, selector) else {
        return false;
    };
    let Ok(item_date) = validate_business_date(date_value) else {
        return false;
    };

    match selector {
        DateFieldSelector::All => match item.date_type.as_deref() {
            Some("deadline") => {
                item_date == target || (include_overdue && item_date < target && !item.completed)
            }
            Some("start_date") | Some("hard_deadline") => item_date == target,
            _ => false,
        },
        DateFieldSelector::One(_) => {
            item_date == target || (include_overdue && item_date < target && !item.completed)
        }
    }
}

fn date_key_in_range(
    item: &DateItem,
    selector: DateFieldSelector,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Option<String> {
    let date_value = relevant_date_for_item(item, selector)?;
    let item_date = validate_business_date(date_value).ok()?;
    if item_date < from || item_date > to {
        return None;
    }
    Some(format_date(&item_date))
}

fn filter_day_summaries(
    summaries: Vec<DaySummary>,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
) -> Vec<DaySummary> {
    summaries
        .into_iter()
        .filter_map(|mut summary| {
            let parsed = validate_business_date(&summary.date).ok()?;
            if parsed < from || parsed > to {
                return None;
            }
            summary.date = format_date(&parsed);
            Some(summary)
        })
        .collect()
}

/// GET /api/items/by-date?date=YYYY-MM-DD&date_field=deadline&include_overdue=true
#[instrument(skip_all, fields(action = "list_items_by_date"))]
pub async fn by_date(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let date = match parse_required_query_date("date", query_param_with_alias(&url, "date", None)) {
        Ok(date) => date,
        Err(resp) => return Ok(resp),
    };

    let include_overdue = url
        .query_pairs()
        .find(|(k, _)| k == "include_overdue")
        .map(|(_, v)| v != "false")
        .unwrap_or(true);

    let date_field_raw = query_param_with_alias(&url, "date_field", Some("field"))
        .unwrap_or_else(|| "deadline".to_string());
    let selector = match parse_date_field_selector(&date_field_raw) {
        Ok(selector) => selector,
        Err(resp) => return Ok(resp),
    };

    let d1 = ctx.env.d1("DB")?;
    let date_str = format_date(&date);

    if matches!(selector, DateFieldSelector::All) {
        // UNION ALL across all three date fields
        let sql = format!(
            "SELECT * FROM ( \
                SELECT {cols}, 'start_date' as date_type, i.completed as sort_completed, l.name as sort_list_name, COALESCE(i.start_time, '') as sort_time, i.position as sort_position \
                FROM items i JOIN lists l ON l.id = i.list_id \
                WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date = ?2 \
                UNION ALL \
                SELECT {cols}, 'deadline' as date_type, i.completed as sort_completed, l.name as sort_list_name, COALESCE(i.deadline_time, '') as sort_time, i.position as sort_position \
                FROM items i JOIN lists l ON l.id = i.list_id \
                WHERE l.user_id = ?1 AND l.archived = 0 \
                AND (i.deadline = ?2{overdue}) \
                UNION ALL \
                SELECT {cols}, 'hard_deadline' as date_type, i.completed as sort_completed, l.name as sort_list_name, '' as sort_time, i.position as sort_position \
                FROM items i JOIN lists l ON l.id = i.list_id \
                WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline = ?2 \
             ) \
             ORDER BY sort_completed ASC, sort_list_name ASC, sort_time ASC, sort_position ASC",
            cols = DATE_ITEM_COLS,
            overdue = if include_overdue {
                " OR (i.deadline < ?2 AND i.completed = 0)"
            } else {
                ""
            },
        );
        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date_str.clone().into()])?
            .all()
            .await?;
        let items = result
            .results::<DateItem>()?
            .into_iter()
            .filter(|item| keep_item_for_day(item, selector, date, include_overdue))
            .collect::<Vec<_>>();
        Response::from_json(&items)
    } else {
        // Single date field query
        let field = match selector {
            DateFieldSelector::One(field) => field,
            DateFieldSelector::All => unreachable!("handled above"),
        };
        let col = field.column_name();

        let sql = if include_overdue {
            format!(
                "SELECT {cols}, '{label}' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND ({col} = ?2 OR ({col} < ?2 AND i.completed = 0)) \
                 ORDER BY i.completed ASC, {col} ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
                label = field.label(),
            )
        } else {
            format!(
                "SELECT {cols}, '{label}' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 AND {col} = ?2 \
                 ORDER BY i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
                label = field.label(),
            )
        };

        let result = d1
            .prepare(&sql)
            .bind(&[user_id.into(), date_str.into()])?
            .all()
            .await?;
        let items = result
            .results::<DateItem>()?
            .into_iter()
            .filter(|item| keep_item_for_day(item, selector, date, include_overdue))
            .collect::<Vec<_>>();
        Response::from_json(&items)
    }
}

/// GET /api/items/calendar?from=YYYY-MM-DD&to=YYYY-MM-DD&date_field=deadline&detail=counts|full
#[instrument(skip_all, fields(action = "list_calendar"))]
pub async fn calendar(req: Request, ctx: RouteContext<String>) -> Result<Response> {
    let user_id = ctx.data.clone();
    let url = req.url()?;

    let from = match parse_required_query_date("from", query_param_with_alias(&url, "from", None)) {
        Ok(from) => from,
        Err(resp) => return Ok(resp),
    };

    let to = match parse_required_query_date("to", query_param_with_alias(&url, "to", None)) {
        Ok(to) => to,
        Err(resp) => return Ok(resp),
    };
    if from > to {
        return validation_error(
            "Invalid query parameters.",
            vec![validation_field("from", "range_start_after_end")],
        );
    }

    let detail = query_param_with_alias(&url, "detail", Some("mode"))
        .unwrap_or_else(|| "counts".to_string());
    if detail != "counts" && detail != "full" {
        return validation_error(
            "Invalid query parameters.",
            vec![validation_field("detail", "invalid_detail")],
        );
    }

    let date_field_raw = query_param_with_alias(&url, "date_field", Some("field"))
        .unwrap_or_else(|| "deadline".to_string());
    let selector = match parse_date_field_selector(&date_field_raw) {
        Ok(selector) => selector,
        Err(resp) => return Ok(resp),
    };

    let d1 = ctx.env.d1("DB")?;
    let from_str = format_date(&from);
    let to_str = format_date(&to);

    if detail == "full" {
        if matches!(selector, DateFieldSelector::All) {
            let sql = format!(
                "SELECT * FROM ( \
                    SELECT {cols}, 'start_date' as date_type, i.start_date as sort_date, i.completed as sort_completed, l.name as sort_list_name, COALESCE(i.start_time, '') as sort_time, i.position as sort_position \
                    FROM items i JOIN lists l ON l.id = i.list_id \
                    WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                    UNION ALL \
                    SELECT {cols}, 'deadline' as date_type, i.deadline as sort_date, i.completed as sort_completed, l.name as sort_list_name, COALESCE(i.deadline_time, '') as sort_time, i.position as sort_position \
                    FROM items i JOIN lists l ON l.id = i.list_id \
                    WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                    UNION ALL \
                    SELECT {cols}, 'hard_deadline' as date_type, i.hard_deadline as sort_date, i.completed as sort_completed, l.name as sort_list_name, '' as sort_time, i.position as sort_position \
                    FROM items i JOIN lists l ON l.id = i.list_id \
                    WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ) \
                 ORDER BY sort_date ASC, sort_completed ASC, sort_list_name ASC, sort_time ASC, sort_position ASC",
                cols = DATE_ITEM_COLS,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[
                    user_id.into(),
                    from_str.clone().into(),
                    to_str.clone().into(),
                ])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            // Group by the date relevant to date_type
            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                if let Some(date_key) = date_key_in_range(&item, selector, from, to) {
                    day_map.entry(date_key).or_default().push(item);
                }
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        } else {
            let field = match selector {
                DateFieldSelector::One(field) => field,
                DateFieldSelector::All => unreachable!("handled above"),
            };
            let col = field.column_name();
            let sql = format!(
                "SELECT {cols}, '{label}' as date_type \
                 FROM items i JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 ORDER BY {col} ASC, i.completed ASC, l.name ASC, i.deadline_time ASC, i.position ASC",
                cols = DATE_ITEM_COLS,
                col = col,
                label = field.label(),
            );
            let result = d1
                .prepare(&sql)
                .bind(&[
                    user_id.into(),
                    from_str.clone().into(),
                    to_str.clone().into(),
                ])?
                .all()
                .await?;
            let items = result.results::<DateItem>()?;

            let mut day_map: std::collections::BTreeMap<String, Vec<DateItem>> =
                std::collections::BTreeMap::new();
            for item in items {
                if let Some(date_key) = date_key_in_range(&item, selector, from, to) {
                    day_map.entry(date_key).or_default().push(item);
                }
            }
            let day_items: Vec<DayItems> = day_map
                .into_iter()
                .map(|(date, items)| DayItems { date, items })
                .collect();

            Response::from_json(&day_items)
        }
    } else {
        // Counts mode
        if matches!(selector, DateFieldSelector::All) {
            let sql = "SELECT date, COUNT(DISTINCT id) as total, \
                 CAST(SUM(CASE WHEN completed = 1 THEN 1 ELSE 0 END) AS INTEGER) as completed \
                 FROM ( \
                     SELECT i.id, i.start_date as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.start_date >= ?2 AND i.start_date <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.deadline >= ?2 AND i.deadline <= ?3 \
                     UNION ALL \
                     SELECT i.id, i.hard_deadline as date, i.completed FROM items i JOIN lists l ON l.id = i.list_id \
                     WHERE l.user_id = ?1 AND l.archived = 0 AND i.hard_deadline >= ?2 AND i.hard_deadline <= ?3 \
                 ) GROUP BY date ORDER BY date ASC";
            let result = d1
                .prepare(sql)
                .bind(&[user_id.into(), from_str.into(), to_str.into()])?
                .all()
                .await?;
            let summaries = filter_day_summaries(result.results::<DaySummary>()?, from, to);
            Response::from_json(&summaries)
        } else {
            let col = match selector {
                DateFieldSelector::One(field) => field.column_name(),
                DateFieldSelector::All => unreachable!("handled above"),
            };
            let sql = format!(
                "SELECT {col} as date, \
                 COUNT(*) as total, \
                 CAST(SUM(i.completed) AS INTEGER) as completed \
                 FROM items i \
                 JOIN lists l ON l.id = i.list_id \
                 WHERE l.user_id = ?1 AND l.archived = 0 \
                 AND {col} >= ?2 AND {col} <= ?3 \
                 GROUP BY {col} \
                 ORDER BY {col} ASC",
                col = col,
            );
            let result = d1
                .prepare(&sql)
                .bind(&[user_id.into(), from_str.into(), to_str.into()])?
                .all()
                .await?;
            let summaries = filter_day_summaries(result.results::<DaySummary>()?, from, to);
            Response::from_json(&summaries)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_deadline_item(deadline: &str, completed: bool) -> DateItem {
        DateItem {
            id: "item-1".into(),
            list_id: "list-1".into(),
            title: "Test".into(),
            description: None,
            completed,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: None,
            start_time: None,
            deadline: Some(deadline.into()),
            deadline_time: None,
            hard_deadline: None,
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            list_name: "List".into(),
            list_type: ListType::Checklist,
            date_type: Some("deadline".into()),
        }
    }

    #[test]
    fn all_selector_uses_start_date_date_type() {
        let item = DateItem {
            id: "item-1".into(),
            list_id: "list-1".into(),
            title: "Start item".into(),
            description: None,
            completed: false,
            position: 0,
            quantity: None,
            actual_quantity: None,
            unit: None,
            start_date: Some("2026-04-12".into()),
            start_time: Some("09:00".into()),
            deadline: None,
            deadline_time: None,
            hard_deadline: None,
            created_at: "2026-04-01T00:00:00Z".into(),
            updated_at: "2026-04-01T00:00:00Z".into(),
            list_name: "List".into(),
            list_type: ListType::Checklist,
            date_type: Some("start_date".into()),
        };

        assert_eq!(
            relevant_date_for_item(&item, DateFieldSelector::All),
            Some("2026-04-12")
        );
        assert!(keep_item_for_day(
            &item,
            DateFieldSelector::All,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 12).unwrap(),
            true,
        ));
    }

    #[test]
    fn keep_item_for_day_matches_exact_date() {
        let item = make_deadline_item("2026-04-10", false);
        let target = chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert!(keep_item_for_day(
            &item,
            DateFieldSelector::One(DateField::Deadline),
            target,
            false
        ));
    }

    #[test]
    fn keep_item_for_day_includes_overdue_incomplete() {
        let item = make_deadline_item("2026-04-08", false);
        let target = chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert!(keep_item_for_day(
            &item,
            DateFieldSelector::One(DateField::Deadline),
            target,
            true
        ));
    }

    #[test]
    fn keep_item_for_day_excludes_overdue_completed() {
        let item = make_deadline_item("2026-04-08", true);
        let target = chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert!(!keep_item_for_day(
            &item,
            DateFieldSelector::One(DateField::Deadline),
            target,
            true
        ));
    }

    #[test]
    fn keep_item_for_day_excludes_overdue_when_flag_false() {
        let item = make_deadline_item("2026-04-08", false);
        let target = chrono::NaiveDate::from_ymd_opt(2026, 4, 10).unwrap();
        assert!(!keep_item_for_day(
            &item,
            DateFieldSelector::One(DateField::Deadline),
            target,
            false
        ));
    }

    #[test]
    fn filter_day_summaries_removes_out_of_range() {
        let summaries = vec![
            DaySummary {
                date: "2026-04-05".into(),
                total: 2,
                completed: 1,
            },
            DaySummary {
                date: "2026-04-10".into(),
                total: 3,
                completed: 0,
            },
            DaySummary {
                date: "2026-04-15".into(),
                total: 1,
                completed: 1,
            },
        ];
        let from = chrono::NaiveDate::from_ymd_opt(2026, 4, 8).unwrap();
        let to = chrono::NaiveDate::from_ymd_opt(2026, 4, 12).unwrap();
        let result = filter_day_summaries(summaries, from, to);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].date, "2026-04-10");
    }

    #[test]
    fn filter_day_summaries_includes_boundary_dates() {
        let summaries = vec![
            DaySummary {
                date: "2026-04-08".into(),
                total: 1,
                completed: 0,
            },
            DaySummary {
                date: "2026-04-12".into(),
                total: 1,
                completed: 1,
            },
        ];
        let from = chrono::NaiveDate::from_ymd_opt(2026, 4, 8).unwrap();
        let to = chrono::NaiveDate::from_ymd_opt(2026, 4, 12).unwrap();
        let result = filter_day_summaries(summaries, from, to);
        assert_eq!(result.len(), 2);
    }
}
