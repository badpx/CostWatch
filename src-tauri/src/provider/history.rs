use rusqlite::OptionalExtension;
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct HistoryPoint {
    pub recorded_at: String,
    pub value: f64,
}

/// Extract the primary numeric value from a ProviderState for history recording.
/// Checks balance first, then available, then returns None.
pub fn get_primary_value(balance: &Option<rust_decimal::Decimal>, available: &Option<rust_decimal::Decimal>) -> Option<f64> {
    if let Some(ref b) = balance {
        return b.to_f64();
    }
    if let Some(ref a) = available {
        return a.to_f64();
    }
    None
}

fn range_to_days(range: &str) -> &str {
    match range {
        "24h" => "-1 days",
        "3d" => "-3 days",
        "1w" => "-7 days",
        "1m" => "-30 days",
        _ => "-7 days",
    }
}

pub fn query_history(provider_id: &str, range: &str) -> Result<Vec<HistoryPoint>, String> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| "home dir not found".to_string())?
        .join(".costwatch")
        .join("history.db");

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("DB open error: {}", e))?;

    let days = range_to_days(range);
    let sql = format!(
        "SELECT recorded_at, value FROM provider_history
         WHERE provider_id = ?1 AND recorded_at >= datetime('now', '{}')
         ORDER BY recorded_at ASC",
        days
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare error: {}", e))?;

    let rows = stmt
        .query_map([provider_id], |row| {
            let recorded_at: String = row.get(0)?;
            let value: f64 = row.get(1)?;
            Ok(HistoryPoint { recorded_at, value })
        })
        .map_err(|e| format!("query error: {}", e))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| format!("row error: {}", e))?);
    }
    Ok(points)
}

pub fn record_history(provider_id: &str, value: f64) -> Result<(), String> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| "home dir not found".to_string())?
        .join(".costwatch")
        .join("history.db");

    let mut conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("DB open error: {}", e))?;

    let tx = conn
        .transaction()
        .map_err(|e| format!("transaction error: {}", e))?;

    let recent: Option<(i64, f64)> = tx
        .query_row(
            "SELECT id, value FROM provider_history
             WHERE provider_id = ?1
             ORDER BY recorded_at DESC LIMIT 1",
            rusqlite::params![provider_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("query recent error: {}", e))?;

    if let Some((id, last_value)) = recent {
        if (last_value - value).abs() < f64::EPSILON {
            tx.execute(
                "UPDATE provider_history SET recorded_at = datetime('now') WHERE id = ?1",
                rusqlite::params![id],
            )
            .map_err(|e| format!("update error: {}", e))?;
            tx.commit().map_err(|e| format!("commit error: {}", e))?;
            return Ok(());
        }
    }

    tx.execute(
        "INSERT INTO provider_history (provider_id, recorded_at, value) VALUES (?1, datetime('now'), ?2)",
        rusqlite::params![provider_id, value],
    )
    .map_err(|e| format!("insert error: {}", e))?;

    tx.commit().map_err(|e| format!("commit error: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[test]
    fn test_range_to_days() {
        assert_eq!(range_to_days("24h"), "-1 days");
        assert_eq!(range_to_days("3d"), "-3 days");
        assert_eq!(range_to_days("1w"), "-7 days");
        assert_eq!(range_to_days("1m"), "-30 days");
        assert_eq!(range_to_days("unknown"), "-7 days");
    }

    #[test]
    fn test_get_primary_value_prefers_balance() {
        let balance = Some(Decimal::new(12345, 2)); // 123.45
        let available = Some(Decimal::new(9999, 2)); // 99.99
        assert_eq!(get_primary_value(&balance, &available), Some(123.45));
    }

    #[test]
    fn test_get_primary_value_falls_back_to_available() {
        let balance: Option<Decimal> = None;
        let available = Some(Decimal::new(7777, 2)); // 77.77
        assert_eq!(get_primary_value(&balance, &available), Some(77.77));
    }

    #[test]
    fn test_get_primary_value_none_when_both_missing() {
        let balance: Option<Decimal> = None;
        let available: Option<Decimal> = None;
        assert_eq!(get_primary_value(&balance, &available), None);
    }
}
