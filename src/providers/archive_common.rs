use serde_json::{json, Value};

pub fn date_only(value: Option<&str>) -> String {
    value.unwrap_or("").chars().take(10).collect()
}

pub fn finished(date: &str, until: &str) -> bool {
    !date.is_empty() && date <= until
}

pub fn archive_race(
    series_key: &str,
    session_key: &str,
    season: i32,
    round: i32,
    name: &str,
    date: &str,
    circuit_name: &str,
    locality: &str,
    country: &str,
    sessions: Value,
) -> Value {
    json!({
        "series_key": series_key,
        "session_key": session_key,
        "event_key": session_key,
        "season": season,
        "round": round,
        "name": name,
        "date": date,
        "circuit_id": "",
        "circuit_name": circuit_name,
        "locality": locality,
        "country": country,
        "lat": Value::Null,
        "lng": Value::Null,
        "has_geometry": false,
        "sessions": sessions,
        "status": "finished"
    })
}

pub fn archive_detail(
    race: Value,
    session_name: &str,
    results: Vec<Value>,
    source: &str,
) -> Value {
    let total_laps = results
        .iter()
        .filter_map(|row| row.get("laps").and_then(|v| v.as_i64()))
        .max()
        .unwrap_or(0)
        .max(1);
    json!({
        "race": race,
        "session": {
            "session_key": race.get("session_key").cloned().unwrap_or(json!("")),
            "type": "race",
            "name": session_name
        },
        "geometry": Value::Null,
        "results": results,
        "laps": [],
        "pit_stops": [],
        "total_laps": total_laps,
        "source": source
    })
}

pub fn archive_years(until: &str) -> Vec<i32> {
    let until_year = until
        .get(0..4)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(2026);
    let current = chrono::Utc::now()
        .format("%Y")
        .to_string()
        .parse::<i32>()
        .unwrap_or(2026);
    let end = until_year.min(current).min(2026);
    let start = (end - 2).max(2022);
    (start..=end).collect()
}
