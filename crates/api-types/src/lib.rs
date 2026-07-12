use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    Anon,
    FreeKey,
    Supporter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub error: ApiError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ApiErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    RateLimited,
    InvalidParam,
    NotFound,
    RangeTooWide,
    Unauthorized,
    LiveDisabled,
    SeriesDisabled,
    EndpointDisabledForSeries,
    LiveDisabledForSeries,
    SeriesNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub active_session_key: Option<i64>,
    pub feed_latency_ms: Option<u64>,
    pub live_redistribution_enabled: bool,
    pub api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Subscribe {
        session_key: SessionKeyRef,
        topics: Vec<String>,
    },
    Unsubscribe {
        topics: Vec<String>,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionKeyRef {
    Key(i64),
    Active(ActiveSession),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActiveSession {
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Snapshot {
        session_key: i64,
        seq: u64,
        state: serde_json::Value,
    },
    Delta {
        seq: u64,
        ts: DateTime<Utc>,
        topic: String,
        patch: serde_json::Value,
    },
    SessionStatus {
        status: SessionLiveStatus,
    },
    Pong,
    Error {
        code: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionLiveStatus {
    Live,
    Finished,
    None,
}

pub mod topics {
    pub const HEARTBEAT: &str = "Heartbeat";
    pub const EXTRAPOLATED_CLOCK: &str = "ExtrapolatedClock";
    pub const TOP_THREE: &str = "TopThree";
    pub const TIMING_STATS: &str = "TimingStats";
    pub const TIMING_APP_DATA: &str = "TimingAppData";
    pub const WEATHER_DATA: &str = "WeatherData";
    pub const TRACK_STATUS: &str = "TrackStatus";
    pub const SESSION_STATUS: &str = "SessionStatus";
    pub const DRIVER_LIST: &str = "DriverList";
    pub const RACE_CONTROL_MESSAGES: &str = "RaceControlMessages";
    pub const SESSION_INFO: &str = "SessionInfo";
    pub const SESSION_DATA: &str = "SessionData";
    pub const LAP_COUNT: &str = "LapCount";
    pub const TIMING_DATA: &str = "TimingData";
    pub const CAR_DATA_Z: &str = "CarData.z";
    pub const POSITION_Z: &str = "Position.z";
}
