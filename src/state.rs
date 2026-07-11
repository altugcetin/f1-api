use api_types::StatusResponse;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: Option<PgPool>,
    pub redis_url: Option<String>,
    pub live_redistribution_enabled: bool,
    pub api_version: String,
}

impl AppState {
    pub fn has_database(&self) -> bool {
        self.db.is_some()
    }

    pub fn has_redis(&self) -> bool {
        self.redis_url.is_some()
    }
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL").ok();
        let db = match database_url {
            Some(url) if !url.is_empty() => {
                let pool = PgPoolOptions::new()
                    .max_connections(10)
                    .connect(&url)
                    .await?;
                Some(pool)
            }
            _ => {
                tracing::warn!("DATABASE_URL unset; running without database");
                None
            }
        };

        let redis_url = std::env::var("REDIS_URL").ok().filter(|u| !u.is_empty());
        let live_redistribution_enabled = std::env::var("LIVE_REDISTRIBUTION_ENABLED")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        Ok(Self {
            db,
            redis_url,
            live_redistribution_enabled,
            api_version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    pub fn status_response(&self) -> StatusResponse {
        StatusResponse {
            active_session_key: None,
            feed_latency_ms: None,
            live_redistribution_enabled: self.live_redistribution_enabled,
            api_version: self.api_version.clone(),
        }
    }
}
