use reqwest::Client;
use std::sync::OnceLock;
use std::time::Duration;

pub const BOT_UA: &str = "alchmist-motorsport-bot/1.0 (+https://f1.alchm.ist/bot)";

pub fn client() -> &'static Client {
    static CLIENT: OnceLock<Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent(BOT_UA)
            .build()
            .unwrap_or_else(|_| Client::new())
    })
}
