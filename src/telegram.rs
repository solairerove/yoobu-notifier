use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tokio::time::sleep;

const MAX_ATTEMPTS: usize = 3;
const BACKOFF_MS: [u64; MAX_ATTEMPTS] = [1_000, 3_000, 9_000];

#[derive(Serialize)]
struct SendMessageBody {
    chat_id: i64,
    text: String,
    parse_mode: String,
}

pub struct TelegramClient {
    client: Client,
}

impl TelegramClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn send_message(&self, bot_token: &str, chat_id: i64, text: &str) -> bool {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
        let body = SendMessageBody {
            chat_id,
            text: text.to_string(),
            parse_mode: "HTML".to_string(),
        };

        for attempt in 0..MAX_ATTEMPTS {
            match self.client.post(&url).json(&body).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        tracing::debug!("sent to chat_id={}", chat_id);
                        return true;
                    }
                    let body_text = response.text().await.unwrap_or_default();
                    if status.as_u16() == 429 {
                        let wait_ms = parse_retry_after(&body_text).unwrap_or(BACKOFF_MS[0]);
                        tracing::warn!(
                            "rate limit 429 chat_id={}, wait {}ms attempt {}/{}",
                            chat_id,
                            wait_ms,
                            attempt + 1,
                            MAX_ATTEMPTS
                        );
                        if attempt < MAX_ATTEMPTS - 1 {
                            sleep(Duration::from_millis(wait_ms)).await;
                        }
                    } else {
                        tracing::warn!(
                            "error {} chat_id={} attempt {}/{}: {}",
                            status,
                            chat_id,
                            attempt + 1,
                            MAX_ATTEMPTS,
                            body_text
                        );
                        if attempt < MAX_ATTEMPTS - 1 {
                            sleep(Duration::from_millis(BACKOFF_MS[attempt])).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "request failed chat_id={} attempt {}/{}: {}",
                        chat_id,
                        attempt + 1,
                        MAX_ATTEMPTS,
                        e
                    );
                    if attempt < MAX_ATTEMPTS - 1 {
                        sleep(Duration::from_millis(BACKOFF_MS[attempt])).await;
                    }
                }
            }
        }

        tracing::warn!("failed after {} attempts chat_id={}", MAX_ATTEMPTS, chat_id);
        false
    }
}

fn parse_retry_after(body: &str) -> Option<u64> {
    let idx = body.find("retry_after")?;
    let after = &body[idx + "retry_after".len()..];
    let digits: String = after
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect();

    digits.parse::<u64>().ok().map(|s| s * 1_000)
}
