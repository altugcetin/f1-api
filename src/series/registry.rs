use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Discipline {
    Circuit,
    Rally,
    Endurance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LegalTier {
    T1,
    T2,
    T3,
    T4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SeriesStatus {
    Active,
    Paused,
    Excluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Coverage {
    Full,
    Live,
    ResultsOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesRecord {
    pub series_key: String,
    pub name: String,
    pub discipline: Discipline,
    pub provider: String,
    pub legal_tier: LegalTier,
    pub live_enabled: bool,
    pub free_delay_seconds: i32,
    pub enabled_endpoints: Vec<String>,
    pub disclaimer_key: String,
    pub status: SeriesStatus,
    pub notes: String,
}

impl SeriesRecord {
    pub fn coverage(&self) -> Coverage {
        match self.legal_tier {
            LegalTier::T3 => Coverage::ResultsOnly,
            LegalTier::T1 | LegalTier::T2 if self.live_enabled => Coverage::Full,
            LegalTier::T1 | LegalTier::T2 => Coverage::Live,
            LegalTier::T4 => Coverage::ResultsOnly,
        }
    }

    pub fn public_view(&self) -> serde_json::Value {
        serde_json::json!({
            "series_key": self.series_key,
            "name": self.name,
            "discipline": self.discipline,
            "coverage": self.coverage(),
            "legal_tier": self.legal_tier,
            "live_enabled": self.live_enabled && self.status == SeriesStatus::Active,
            "free_delay_seconds": self.free_delay_seconds,
            "enabled_endpoints": self.enabled_endpoints,
            "disclaimer_key": self.disclaimer_key,
            "disclaimer": crate::disclaimers::text_for(&self.disclaimer_key),
            "status": self.status,
        })
    }
}

fn circuit_live_endpoints() -> Vec<String> {
    vec![
        "events".into(),
        "sessions".into(),
        "entries".into(),
        "competitors".into(),
        "results".into(),
        "standings".into(),
        "laps".into(),
        "stints".into(),
        "pit".into(),
        "intervals".into(),
        "position".into(),
        "race_control".into(),
        "weather".into(),
    ]
}

fn circuit_results_endpoints() -> Vec<String> {
    vec![
        "events".into(),
        "sessions".into(),
        "entries".into(),
        "competitors".into(),
        "results".into(),
        "standings".into(),
    ]
}

fn endurance_results_endpoints() -> Vec<String> {
    vec![
        "events".into(),
        "sessions".into(),
        "entries".into(),
        "competitors".into(),
        "results".into(),
        "standings".into(),
    ]
}

fn rally_endpoints() -> Vec<String> {
    vec![
        "events".into(),
        "sessions".into(),
        "entries".into(),
        "competitors".into(),
        "results".into(),
        "standings".into(),
        "itinerary".into(),
        "stages".into(),
        "stage_times".into(),
        "split_times".into(),
        "overall".into(),
        "penalties".into(),
        "retirements".into(),
    ]
}

fn f1_endpoints() -> Vec<String> {
    let mut endpoints = circuit_live_endpoints();
    endpoints.push("location".into());
    endpoints.push("car_data".into());
    endpoints.push("archive".into());
    endpoints.push("schedule".into());
    endpoints.push("circuits".into());
    endpoints
}

fn seed() -> Vec<SeriesRecord> {
    vec![
        SeriesRecord {
            series_key: "f1".into(),
            name: "Formula 1".into(),
            discipline: Discipline::Circuit,
            provider: "f1-open".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: f1_endpoints(),
            disclaimer_key: "f1".into(),
            status: SeriesStatus::Active,
            notes: "Existing v2 behaviour; classified t2".into(),
        },
        SeriesRecord {
            series_key: "f2".into(),
            name: "FIA Formula 2".into(),
            discipline: Discipline::Circuit,
            provider: "f2f3-open".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "f2".into(),
            status: SeriesStatus::Paused,
            notes: "Protocol discovery pending; paused until verified".into(),
        },
        SeriesRecord {
            series_key: "f3".into(),
            name: "FIA Formula 3".into(),
            discipline: Discipline::Circuit,
            provider: "f2f3-open".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "f3".into(),
            status: SeriesStatus::Paused,
            notes: "Protocol discovery pending; paused until verified".into(),
        },
        SeriesRecord {
            series_key: "motogp".into(),
            name: "MotoGP".into(),
            discipline: Discipline::Circuit,
            provider: "pulselive".into(),
            legal_tier: LegalTier::T1,
            live_enabled: true,
            free_delay_seconds: 30,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "motogp".into(),
            status: SeriesStatus::Active,
            notes: "PulseLive public results; live poll in private feed worker".into(),
        },
        SeriesRecord {
            series_key: "moto2".into(),
            name: "Moto2".into(),
            discipline: Discipline::Circuit,
            provider: "pulselive".into(),
            legal_tier: LegalTier::T1,
            live_enabled: true,
            free_delay_seconds: 30,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "motogp".into(),
            status: SeriesStatus::Active,
            notes: String::new(),
        },
        SeriesRecord {
            series_key: "moto3".into(),
            name: "Moto3".into(),
            discipline: Discipline::Circuit,
            provider: "pulselive".into(),
            legal_tier: LegalTier::T1,
            live_enabled: true,
            free_delay_seconds: 30,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "motogp".into(),
            status: SeriesStatus::Active,
            notes: String::new(),
        },
        SeriesRecord {
            series_key: "motoe".into(),
            name: "MotoE".into(),
            discipline: Discipline::Circuit,
            provider: "pulselive".into(),
            legal_tier: LegalTier::T1,
            live_enabled: true,
            free_delay_seconds: 30,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "motogp".into(),
            status: SeriesStatus::Active,
            notes: String::new(),
        },
        SeriesRecord {
            series_key: "wrc".into(),
            name: "WRC".into(),
            discipline: Discipline::Rally,
            provider: "wrc-results".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: rally_endpoints(),
            disclaimer_key: "wrc".into(),
            status: SeriesStatus::Active,
            notes: "Stage polling, not a push feed".into(),
        },
        SeriesRecord {
            series_key: "nascar-cup".into(),
            name: "NASCAR Cup Series".into(),
            discipline: Discipline::Circuit,
            provider: "nascar-feeds".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "nascar".into(),
            status: SeriesStatus::Paused,
            notes: "Paused pending DG-1 verification".into(),
        },
        SeriesRecord {
            series_key: "indycar".into(),
            name: "INDYCAR".into(),
            discipline: Discipline::Circuit,
            provider: "indycar-racecontrol".into(),
            legal_tier: LegalTier::T2,
            live_enabled: true,
            free_delay_seconds: 45,
            enabled_endpoints: circuit_live_endpoints(),
            disclaimer_key: "indycar".into(),
            status: SeriesStatus::Paused,
            notes: "Paused pending DG-2 verification".into(),
        },
        SeriesRecord {
            series_key: "formula-e".into(),
            name: "Formula E".into(),
            discipline: Discipline::Circuit,
            provider: "pulselive-fe".into(),
            legal_tier: LegalTier::T2,
            live_enabled: false,
            free_delay_seconds: 45,
            enabled_endpoints: circuit_results_endpoints(),
            disclaimer_key: "formula-e".into(),
            status: SeriesStatus::Active,
            notes: "Results and calendar only; live permanently disabled".into(),
        },
        SeriesRecord {
            series_key: "wec".into(),
            name: "FIA WEC".into(),
            discipline: Discipline::Endurance,
            provider: "manual-facts".into(),
            legal_tier: LegalTier::T3,
            live_enabled: false,
            free_delay_seconds: 0,
            enabled_endpoints: endurance_results_endpoints(),
            disclaimer_key: "wec".into(),
            status: SeriesStatus::Active,
            notes: "Results-only facts; no timing provider feeds".into(),
        },
        SeriesRecord {
            series_key: "imsa".into(),
            name: "IMSA".into(),
            discipline: Discipline::Endurance,
            provider: "manual-facts".into(),
            legal_tier: LegalTier::T3,
            live_enabled: false,
            free_delay_seconds: 0,
            enabled_endpoints: endurance_results_endpoints(),
            disclaimer_key: "imsa".into(),
            status: SeriesStatus::Active,
            notes: String::new(),
        },
        SeriesRecord {
            series_key: "elms".into(),
            name: "European Le Mans Series".into(),
            discipline: Discipline::Endurance,
            provider: "manual-facts".into(),
            legal_tier: LegalTier::T3,
            live_enabled: false,
            free_delay_seconds: 0,
            enabled_endpoints: endurance_results_endpoints(),
            disclaimer_key: "wec".into(),
            status: SeriesStatus::Active,
            notes: String::new(),
        },
        SeriesRecord {
            series_key: "nls".into(),
            name: "NLS / N24".into(),
            discipline: Discipline::Endurance,
            provider: "manual-facts".into(),
            legal_tier: LegalTier::T3,
            live_enabled: false,
            free_delay_seconds: 0,
            enabled_endpoints: endurance_results_endpoints(),
            disclaimer_key: "nls".into(),
            status: SeriesStatus::Active,
            notes: "Includes N24".into(),
        },
        SeriesRecord {
            series_key: "gtwc-europe".into(),
            name: "GT World Challenge Europe".into(),
            discipline: Discipline::Endurance,
            provider: "manual-facts".into(),
            legal_tier: LegalTier::T3,
            live_enabled: false,
            free_delay_seconds: 0,
            enabled_endpoints: endurance_results_endpoints(),
            disclaimer_key: "gtwc".into(),
            status: SeriesStatus::Active,
            notes: "Includes Spa 24h".into(),
        },
    ]
}

static REGISTRY: OnceLock<HashMap<String, SeriesRecord>> = OnceLock::new();

fn map() -> &'static HashMap<String, SeriesRecord> {
    REGISTRY.get_or_init(|| {
        seed()
            .into_iter()
            .map(|row| (row.series_key.clone(), row))
            .collect()
    })
}

pub fn all() -> Vec<&'static SeriesRecord> {
    let mut rows: Vec<_> = map().values().collect();
    rows.sort_by(|a, b| a.series_key.cmp(&b.series_key));
    rows
}

pub fn get(series_key: &str) -> Option<&'static SeriesRecord> {
    map().get(series_key)
}

pub fn active_public() -> Vec<&'static SeriesRecord> {
    all()
        .into_iter()
        .filter(|row| row.status != SeriesStatus::Excluded)
        .collect()
}
