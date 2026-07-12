use serde_json::{json, Value};
use std::time::Duration;

use crate::providers::http::client;
use crate::response_cache;

const PAGE: &str = "2026_IndyCar_Series";

async fn get_json(cache_key: &str, ttl: Duration, url: &str) -> Value {
    if let Some(cached) = response_cache::get(cache_key, ttl) {
        return cached;
    }
    let Ok(response) = client().get(url).send().await else {
        return json!({});
    };
    if !response.status().is_success() {
        return json!({});
    }
    let Ok(body) = response.json::<Value>().await else {
        return json!({});
    };
    response_cache::set(cache_key.to_string(), body.clone());
    body
}

fn clean_wiki_cell(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    // [[Link|Label]] or [[Link]]
    while let Some(start) = s.find("[[") {
        let Some(end_rel) = s[start..].find("]]") else {
            break;
        };
        let end = start + end_rel;
        let inner = s[start + 2..end].to_string();
        let label = inner
            .split('|')
            .next_back()
            .unwrap_or(inner.as_str())
            .trim()
            .to_string();
        s.replace_range(start..=end + 1, &label);
    }
    // {{color box|...|S|...}} style markers -> drop
    while let Some(start) = s.find("{{") {
        let Some(end_rel) = s[start..].find("}}") else {
            break;
        };
        let end = start + end_rel;
        s.replace_range(start..=end + 1, "");
    }
    s = s.replace("<br />", " ").replace("<br/>", " ").replace("<br>", " ");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn parse_schedule_wikitext(wikitext: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    let mut in_race_table = false;
    let mut current: Option<Vec<String>> = None;

    for line in wikitext.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("{|") && trimmed.contains("wikitable") {
            in_race_table = false;
            current = None;
            continue;
        }
        if trimmed.starts_with("!Rd") || trimmed.starts_with("! Rd") {
            in_race_table = true;
            continue;
        }
        if !in_race_table {
            continue;
        }
        if trimmed.starts_with("|}") {
            if let Some(cells) = current.take() {
                if let Some(event) = row_to_event(&cells) {
                    rows.push(event);
                }
            }
            break;
        }
        if trimmed.starts_with('!')
            && trimmed
                .chars()
                .nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            if let Some(cells) = current.take() {
                if let Some(event) = row_to_event(&cells) {
                    rows.push(event);
                }
            }
            let round = trimmed.trim_start_matches('!').trim();
            current = Some(vec![round.to_string()]);
            continue;
        }
        if trimmed == "|-" {
            if let Some(cells) = current.take() {
                if let Some(event) = row_to_event(&cells) {
                    rows.push(event);
                }
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix('|') {
            if let Some(cells) = current.as_mut() {
                cells.push(clean_wiki_cell(rest));
            }
        }
    }
    if let Some(cells) = current.take() {
        if let Some(event) = row_to_event(&cells) {
            rows.push(event);
        }
    }
    rows
}

fn row_to_event(cells: &[String]) -> Option<Value> {
    // Rd., Date, Race name, Track, Location, Time
    if cells.len() < 5 {
        return None;
    }
    let round: i64 = cells[0].parse().ok()?;
    let date_label = cells[1].clone();
    let name = cells[2].clone();
    let track = cells[3].clone();
    let location = cells[4].clone();
    if name.is_empty() || name.eq_ignore_ascii_case("date") {
        return None;
    }
    let date_start = rough_date_2026(&date_label);
    Some(json!({
        "series_key": "indycar",
        "event_key": format!("2026-r{round}"),
        "name": name,
        "circuit_name": track,
        "locality": location,
        "country": "USA",
        "date_start": date_start,
        "date_end": date_start,
        "round": round,
        "date_label": date_label,
        "source_id": "wikipedia:2026_IndyCar_Series:schedule"
    }))
}

fn rough_date_2026(label: &str) -> String {
    // "March 1" / "May 24" / "August 8–9" -> 2026-03-01
    let cleaned = label
        .split(['–', '-', '/'])
        .next()
        .unwrap_or(label)
        .trim();
    let parts: Vec<&str> = cleaned.split_whitespace().collect();
    if parts.len() < 2 {
        return format!("2026-{cleaned}");
    }
    let month = match parts[0].to_lowercase().as_str() {
        "january" => 1,
        "february" => 2,
        "march" => 3,
        "april" => 4,
        "may" => 5,
        "june" => 6,
        "july" => 7,
        "august" => 8,
        "september" => 9,
        "october" => 10,
        "november" => 11,
        "december" => 12,
        _ => 0,
    };
    let day: u32 = parts[1]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(1);
    if month == 0 {
        return format!("2026-{cleaned}");
    }
    format!("2026-{month:02}-{day:02}")
}

fn extract_wiki_link_label(raw: &str) -> Option<String> {
    let start = raw.find("[[")?;
    let end = raw[start..].find("]]")? + start;
    let inner = &raw[start + 2..end];
    let label = inner.split('|').next_back().unwrap_or(inner).trim();
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

fn parse_standings_wikitext(wikitext: &str) -> Vec<Value> {
    let mut rows = Vec::new();
    let mut current_pos: Option<i64> = None;
    let mut current_driver: Option<String> = None;
    let mut last_bang_number: Option<f64> = None;

    for line in wikitext.lines() {
        let trimmed = line.trim();
        if trimmed == "|-" || trimmed.starts_with("|}") {
            if let (Some(position), Some(driver)) = (current_pos, current_driver.clone()) {
                if let Some(points) = last_bang_number {
                    let (given, family) = if let Some((g, f)) = driver.split_once(' ') {
                        (g.to_string(), f.to_string())
                    } else {
                        (driver.clone(), String::new())
                    };
                    rows.push(json!({
                        "position": position,
                        "points": points,
                        "wins": Value::Null,
                        "driver_id": driver.to_lowercase().replace(' ', "-"),
                        "tla": family.chars().take(3).collect::<String>().to_uppercase(),
                        "given_name": given,
                        "family_name": family,
                        "constructor_id": "",
                        "constructor_name": "",
                    }));
                }
            }
            current_pos = None;
            current_driver = None;
            last_bang_number = None;
            if trimmed.starts_with("|}") {
                // Keep scanning; nested tables may close early.
                continue;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix('!') {
            let value = clean_wiki_cell(rest);
            let digits: String = value
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if let Ok(num) = digits.parse::<f64>() {
                if current_pos.is_none() && current_driver.is_none() && num.fract() == 0.0 && num < 100.0 {
                    current_pos = Some(num as i64);
                } else if current_driver.is_some() {
                    last_bang_number = Some(num);
                }
            }
            continue;
        }

        if current_driver.is_none() {
            if let Some(name) = extract_wiki_link_label(trimmed) {
                if !name.eq_ignore_ascii_case("driver")
                    && !name.contains("Grand Prix")
                    && !name.contains("Indy")
                    && name.split_whitespace().count() <= 4
                {
                    current_driver = Some(name);
                }
            }
        }
    }
    rows
}

async fn section_wikitext(section: &str, cache_key: &str) -> String {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=parse&page={PAGE}&prop=wikitext&section={section}&format=json"
    );
    let body = get_json(cache_key, Duration::from_secs(6 * 3600), &url).await;
    body.pointer("/parse/wikitext/*")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

async fn find_section_index(needle: &str) -> Option<String> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?action=parse&page={PAGE}&prop=sections&format=json"
    );
    let body = get_json("indycar:sections", Duration::from_secs(6 * 3600), &url).await;
    let sections = body
        .pointer("/parse/sections")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let needle_l = needle.to_lowercase();
    sections
        .iter()
        .find(|row| {
            row.get("line")
                .and_then(|v| v.as_str())
                .map(|line| line.eq_ignore_ascii_case(needle))
                .unwrap_or(false)
        })
        .or_else(|| {
            sections.iter().find(|row| {
                row.get("line")
                    .and_then(|v| v.as_str())
                    .map(|line| line.to_lowercase() == needle_l)
                    .unwrap_or(false)
            })
        })
        .or_else(|| {
            sections.iter().find(|row| {
                row.get("line")
                    .and_then(|v| v.as_str())
                    .map(|line| line.to_lowercase().starts_with(&needle_l))
                    .unwrap_or(false)
            })
        })
        .and_then(|row| row.get("index").and_then(|v| v.as_str()).map(|s| s.to_string()))
}

pub async fn events() -> Value {
    let section = find_section_index("schedule")
        .await
        .unwrap_or_else(|| "10".into());
    let wikitext = section_wikitext(&section, &format!("indycar:schedule:{section}")).await;
    Value::Array(parse_schedule_wikitext(&wikitext))
}

pub async fn sessions() -> Value {
    let events = events().await;
    let mapped: Vec<Value> = events
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "series_key": "indycar",
                "session_key": row.get("event_key").cloned(),
                "event_key": row.get("event_key").cloned(),
                "name": "Race",
                "type": "RACE",
                "date_start": row.get("date_start").cloned(),
                "source_id": "wikipedia:indycar:sessions"
            })
        })
        .collect();
    Value::Array(mapped)
}

pub async fn standings() -> Value {
    let section = find_section_index("driver standings")
        .await
        .or(find_section_index("points standings").await)
        .unwrap_or_else(|| "18".into());
    let wikitext = section_wikitext(&section, &format!("indycar:standings:{section}")).await;
    let rows = parse_standings_wikitext(&wikitext);
    json!({
        "series_key": "indycar",
        "season": 2026,
        "standings": rows,
        "source_id": format!("wikipedia:2026_IndyCar_Series:standings:{section}")
    })
}

pub async fn results(event_key: Option<&str>) -> Value {
    let events = events().await;
    let event = events
        .as_array()
        .into_iter()
        .flatten()
        .find(|row| {
            event_key
                .map(|key| row.get("event_key").and_then(|v| v.as_str()) == Some(key))
                .unwrap_or(false)
        })
        .cloned();
    let Some(event) = event else {
        return json!([]);
    };
    json!({
        "series_key": "indycar",
        "event_key": event.get("event_key").cloned(),
        "name": event.get("name").cloned(),
        "circuit_name": event.get("circuit_name").cloned(),
        "country": event.get("country").cloned(),
        "date_start": event.get("date_start").cloned(),
        "round": event.get("round").cloned(),
        "sessions": [{
            "session_key": event.get("event_key").cloned(),
            "name": "Race",
            "type": "RACE",
            "classification": []
        }],
        "note": "Public classification tables are ingested from multi-source records; schedule and standings are served now.",
        "source_id": "wikipedia:indycar:results"
    })
}

pub async fn entries() -> Value {
    let standings = standings().await;
    let rows: Vec<Value> = standings
        .get("standings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            json!({
                "full_name": format!(
                    "{} {}",
                    row.get("given_name").and_then(|v| v.as_str()).unwrap_or(""),
                    row.get("family_name").and_then(|v| v.as_str()).unwrap_or("")
                ).trim(),
                "name_acronym": row.get("tla").cloned(),
                "team_name": row.get("constructor_name").cloned(),
                "broadcast_name": row.get("family_name").cloned(),
            })
        })
        .collect();
    Value::Array(rows)
}
