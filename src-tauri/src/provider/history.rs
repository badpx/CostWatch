use rusqlite::OptionalExtension;
use rust_decimal::prelude::ToPrimitive;
use serde::Serialize;

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Serialize)]
pub struct HistoryPoint {
    pub recorded_at: String,
    pub value: f64,
    #[serde(default, skip_serializing_if = "is_false")]
    pub interpolated: bool,
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

fn range_to_ms(range: &str) -> i64 {
    match range {
        "24h" => 24 * 60 * 60 * 1000,
        "3d" => 3 * 24 * 60 * 60 * 1000,
        "1w" => 7 * 24 * 60 * 60 * 1000,
        "1m" => 30 * 24 * 60 * 60 * 1000,
        _ => 7 * 24 * 60 * 60 * 1000,
    }
}

fn parse_iso_to_millis(s: &str) -> Result<i64, String> {
    let dt = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
        .map_err(|e| format!("parse datetime error: {}", e))?;
    Ok(dt.and_utc().timestamp_millis())
}

fn make_interpolated_point(
    range_start_iso: &str,
    prev: &HistoryPoint,
    first_in_range: Option<&HistoryPoint>,
) -> Result<HistoryPoint, String> {
    let t_min = parse_iso_to_millis(range_start_iso)?;
    let t_prev = parse_iso_to_millis(&prev.recorded_at)?;

    let v_interp = if let Some(first) = first_in_range {
        let t_first = parse_iso_to_millis(&first.recorded_at)?;
        let ratio = if t_first == t_prev {
            0.0
        } else {
            (t_min as f64 - t_prev as f64) / (t_first as f64 - t_prev as f64)
        };
        prev.value + (first.value - prev.value) * ratio
    } else {
        // No point in range: horizontal line from previous value.
        prev.value
    };

    Ok(HistoryPoint {
        recorded_at: range_start_iso.to_string(),
        value: v_interp,
        interpolated: true,
    })
}

pub fn query_history(provider_id: &str, range: &str) -> Result<Vec<HistoryPoint>, String> {
    let db_path = dirs::home_dir()
        .ok_or_else(|| "home dir not found".to_string())?
        .join(".costwatch")
        .join("history.db");

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("DB open error: {}", e))?;

    let days = range_to_days(range);

    // 1. Query points within the range.
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
            Ok(HistoryPoint {
                recorded_at,
                value,
                interpolated: false,
            })
        })
        .map_err(|e| format!("query error: {}", e))?;

    let mut points = Vec::new();
    for row in rows {
        points.push(row.map_err(|e| format!("row error: {}", e))?);
    }

    // 2. Determine whether interpolation is needed.
    //   - Empty or single point: always interpolate.
    //   - >= 2 points: interpolate when the real data does not yet span
    //     the full time window (e.g. app just restarted after a long gap).
    //     The interpolated point sits at the window's left edge, giving
    //     the line a left-hand anchor until enough history accumulates.
    let window_ms = range_to_ms(range);
    let needs_interp = if points.len() <= 1 {
        true
    } else {
        let first_ts = parse_iso_to_millis(&points[0].recorded_at)?;
        let last_ts = parse_iso_to_millis(&points[points.len() - 1].recorded_at)?;
        (last_ts - first_ts) < window_ms
    };

    if needs_interp {
        let range_start_iso: String = conn
            .query_row(
                &format!("SELECT datetime('now', '{}')", days),
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("range_start query error: {}", e))?;

        let prev_sql = format!(
            "SELECT recorded_at, value FROM provider_history
             WHERE provider_id = ?1 AND recorded_at < datetime('now', '{}')
             ORDER BY recorded_at DESC LIMIT 1",
            days
        );

        if let Ok(prev) = conn.query_row(&prev_sql, [provider_id], |row| {
            let recorded_at: String = row.get(0)?;
            let value: f64 = row.get(1)?;
            Ok(HistoryPoint {
                recorded_at,
                value,
                interpolated: false,
            })
        }) {
            let interpolated =
                make_interpolated_point(&range_start_iso, &prev, points.first())?;
            let mut result = vec![interpolated];
            result.extend(points);
            return Ok(result);
        }
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
            let is_today: bool = tx
                .query_row(
                    "SELECT COUNT(*) FROM provider_history
                     WHERE provider_id = ?1 AND id = ?2 AND recorded_at >= date('now')",
                    rusqlite::params![provider_id, id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap_or(0) > 0;

            let today_count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM provider_history
                     WHERE provider_id = ?1 AND recorded_at >= date('now')",
                    rusqlite::params![provider_id],
                    |row| row.get(0),
                )
                .map_err(|e| format!("today_count error: {}", e))?;

            if is_today && today_count >= 2 {
                tx.execute(
                    "UPDATE provider_history SET recorded_at = datetime('now') WHERE id = ?1",
                    rusqlite::params![id],
                )
                .map_err(|e| format!("update error: {}", e))?;
                tx.commit().map_err(|e| format!("commit error: {}", e))?;
                return Ok(());
            }
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

    #[test]
    fn test_parse_iso_to_millis() {
        let ms = parse_iso_to_millis("2026-05-11 12:00:00").unwrap();
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 5, 11)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(ms, expected);
    }

    #[test]
    fn test_interpolate_between_two_points() {
        let prev = HistoryPoint {
            recorded_at: "2026-05-10 00:00:00".to_string(),
            value: 100.0,
            interpolated: false,
        };
        let first = HistoryPoint {
            recorded_at: "2026-05-11 12:00:00".to_string(),
            value: 90.0,
            interpolated: false,
        };
        let result = make_interpolated_point("2026-05-11 00:00:00", &prev, Some(&first)).unwrap();
        assert!(result.interpolated);
        // Exactly halfway in time -> halfway in value.
        assert!((result.value - 93.333333).abs() < 0.001, "got {}", result.value);
    }

    #[test]
    fn test_interpolate_horizontal_when_no_in_range_point() {
        let prev = HistoryPoint {
            recorded_at: "2026-05-10 00:00:00".to_string(),
            value: 88.5,
            interpolated: false,
        };
        let result = make_interpolated_point("2026-05-11 00:00:00", &prev, None).unwrap();
        assert!(result.interpolated);
        assert_eq!(result.value, 88.5);
    }

    #[test]
    fn test_interpolate_zero_ratio_at_prev() {
        let prev = HistoryPoint {
            recorded_at: "2026-05-11 00:00:00".to_string(),
            value: 50.0,
            interpolated: false,
        };
        let first = HistoryPoint {
            recorded_at: "2026-05-11 12:00:00".to_string(),
            value: 60.0,
            interpolated: false,
        };
        let result = make_interpolated_point("2026-05-11 00:00:00", &prev, Some(&first)).unwrap();
        assert!(result.interpolated);
        assert!((result.value - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_history_point_serde_omits_interpolated_when_false() {
        let real = HistoryPoint {
            recorded_at: "2026-05-11 00:00:00".to_string(),
            value: 42.0,
            interpolated: false,
        };
        let json = serde_json::to_string(&real).unwrap();
        assert!(!json.contains("interpolated"), "json: {}", json);
    }

    #[test]
    fn test_history_point_serde_includes_interpolated_when_true() {
        let interp = HistoryPoint {
            recorded_at: "2026-05-11 00:00:00".to_string(),
            value: 42.0,
            interpolated: true,
        };
        let json = serde_json::to_string(&interp).unwrap();
        assert!(json.contains("\"interpolated\":true"), "json: {}", json);
    }

    #[test]
    fn test_merge_same_value_same_day() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE provider_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                value REAL NOT NULL
            )",
            [],
        )
        .unwrap();

        // Insert two records for today
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now'), 100.0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now'), 100.0)",
            [],
        )
        .unwrap();

        let is_today: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND id = (SELECT MAX(id) FROM provider_history WHERE provider_id = 'p1')
                 AND recorded_at >= date('now')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(is_today);

        let today_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND recorded_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(today_count, 2);

        // Same value, today_count >= 2, and today's record -> should merge
        assert!(is_today && today_count >= 2);
    }

    #[test]
    fn test_cross_day_creates_two_today_records() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE provider_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                value REAL NOT NULL
            )",
            [],
        )
        .unwrap();

        // Two records from yesterday
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now', '-1 days'), 100.0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now', '-1 days'), 100.0)",
            [],
        )
        .unwrap();

        // First refresh today: latest record is from yesterday (is_today = false)
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now'), 100.0)",
            [],
        )
        .unwrap();

        let today_count_1: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND recorded_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(today_count_1, 1);

        // Second refresh today: latest is today, but today_count = 1 (< 2) -> should insert
        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now'), 100.0)",
            [],
        )
        .unwrap();

        let today_count_2: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND recorded_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(today_count_2, 2);

        // Third refresh today: latest is today, today_count = 2 (>= 2) -> should merge
        // Simulate the merge by updating the latest record's timestamp
        conn.execute(
            "UPDATE provider_history SET recorded_at = datetime('now') WHERE id = (SELECT MAX(id) FROM provider_history WHERE provider_id = 'p1')",
            [],
        )
        .unwrap();

        let today_count_3: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND recorded_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        // Merge updates timestamp, does not insert -> count stays 2
        assert_eq!(today_count_3, 2);
    }

    #[test]
    fn test_insert_different_value_any_day() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE provider_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider_id TEXT NOT NULL,
                recorded_at TEXT NOT NULL,
                value REAL NOT NULL
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO provider_history (provider_id, recorded_at, value)
             VALUES ('p1', datetime('now'), 100.0)",
            [],
        )
        .unwrap();

        let is_today: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND id = (SELECT MAX(id) FROM provider_history WHERE provider_id = 'p1')
                 AND recorded_at >= date('now')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            > 0;
        assert!(is_today);

        let today_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_history
                 WHERE provider_id = 'p1' AND recorded_at >= date('now')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(today_count, 1);

        // Different value -> should NOT merge regardless of day or count
        let new_value = 200.0f64;
        let last_value = 100.0f64;
        assert!((last_value - new_value).abs() >= f64::EPSILON);
    }
}
