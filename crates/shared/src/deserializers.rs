use serde::{Deserialize, Deserializer};

pub(crate) fn bool_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Bool(b) => Ok(b),
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) != 0.0),
        _ => Ok(false),
    }
}

pub(crate) fn optional_bool_from_number<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<bool>, D::Error> {
    let v = Option::<serde_json::Value>::deserialize(d)?;
    match v {
        Some(serde_json::Value::Bool(b)) => Ok(Some(b)),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.as_f64().unwrap_or(0.0) != 0.0)),
        Some(serde_json::Value::Null) | None => Ok(None),
        _ => Ok(None),
    }
}

pub(crate) fn u32_from_number<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::Number(n) => Ok(n.as_f64().unwrap_or(0.0) as u32),
        _ => Ok(0),
    }
}

pub(crate) fn features_from_json<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<crate::models::list::ListFeature>, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    match v {
        serde_json::Value::String(s) => serde_json::from_str(&s).map_err(serde::de::Error::custom),
        serde_json::Value::Array(_) => serde_json::from_value(v).map_err(serde::de::Error::custom),
        serde_json::Value::Null => Ok(vec![]),
        _ => Ok(vec![]),
    }
}

pub(crate) fn default_config() -> serde_json::Value {
    serde_json::json!({})
}

pub(crate) fn double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::<T>::deserialize(d)?))
}
