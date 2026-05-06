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

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("DB open error: {}", e))?;

    // Skip if a record was inserted for this provider within the last 30 seconds
    // to avoid duplicates from concurrent fetch paths (e.g. refresh_all + test_connection).
    let recent: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_history WHERE provider_id = ?1 AND recorded_at >= datetime('now', '-30 seconds')",
            rusqlite::params![provider_id],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if recent > 0 {
        return Ok(());
    }

    conn.execute(
        "INSERT INTO provider_history (provider_id, recorded_at, value) VALUES (?1, datetime('now'), ?2)",
        rusqlite::params![provider_id, value],
    )
    .map_err(|e| format!("insert error: {}", e))?;

    Ok(())
}
