use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::OnceLock;

fn facts() -> &'static HashMap<&'static str, Value> {
    static FACTS: OnceLock<HashMap<&'static str, Value>> = OnceLock::new();
    FACTS.get_or_init(|| {
        let mut out = HashMap::new();
        out.insert(
            "wec",
            json!({
                "series_key": "wec",
                "note": "Seed facts only. Production rows require multi-source public classification records.",
                "results": [],
                "sources": [],
                "source_id": "manual-facts:wec"
            }),
        );
        out.insert(
            "imsa",
            json!({
                "series_key": "imsa",
                "note": "Seed facts only. Production rows require multi-source public classification records.",
                "results": [],
                "sources": [],
                "source_id": "manual-facts:imsa"
            }),
        );
        out.insert(
            "elms",
            json!({
                "series_key": "elms",
                "note": "Seed facts only.",
                "results": [],
                "sources": [],
                "source_id": "manual-facts:elms"
            }),
        );
        out.insert(
            "nls",
            json!({
                "series_key": "nls",
                "note": "Seed facts only. Includes N24 when entered.",
                "results": [],
                "sources": [],
                "source_id": "manual-facts:nls"
            }),
        );
        out.insert(
            "gtwc-europe",
            json!({
                "series_key": "gtwc-europe",
                "note": "Seed facts only. Includes Spa 24h when entered.",
                "results": [],
                "sources": [],
                "source_id": "manual-facts:gtwc-europe"
            }),
        );
        out
    })
}

pub fn for_series(series_key: &str) -> Value {
    facts()
        .get(series_key)
        .cloned()
        .unwrap_or_else(|| {
            json!({
                "series_key": series_key,
                "results": [],
                "sources": [],
                "source_id": format!("manual-facts:{series_key}")
            })
        })
}
