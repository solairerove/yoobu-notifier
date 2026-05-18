use crate::telegram::TelegramClient;
use serde_json::Value;
use sqlx::{PgPool, Row};

pub struct Processor {
    pool: PgPool,
    telegram_client: TelegramClient,
}

impl Processor {
    pub fn new(pool: PgPool, telegram_client: TelegramClient) -> Self {
        Self {
            pool,
            telegram_client,
        }
    }

    pub async fn run(&self) {
        let mut listener = sqlx::postgres::PgListener::connect_with(&self.pool)
            .await
            .expect("failed to create pg listener");

        listener
            .listen("notification_outbox")
            .await
            .expect("failed to listen");

        tracing::info!("listening on notification_outbox channel");

        self.process_pending().await;

        loop {
            match listener.recv().await {
                Ok(_) => self.process_pending().await,
                Err(e) => {
                    tracing::error!("pg listener error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_pending(&self) {
        let rows = sqlx::query(
            "SELECT id, tenant_id, event_type, payload
                FROM notification_outbox
                WHERE processed_at IS NULL
                ORDER BY created_at ASC",
        )
        .fetch_all(&self.pool)
        .await;

        match rows {
            Ok(rows) => {
                for row in rows {
                    let id: i64 = row.get("id");
                    let tenant_id: i64 = row.get("tenant_id");
                    let event_type: String = row.get("event_type");
                    let payload: Value = row.get("payload");
                    self.process_entry(id, tenant_id, &event_type, &payload)
                        .await;
                }
            }
            Err(e) => tracing::error!("fetch outbox failed: {}", e),
        }
    }

    async fn process_entry(&self, id: i64, tenant_id: i64, event_type: &str, payload: &Value) {
        let bot_token = self.fetch_bot_token(tenant_id).await;
        let bot_token = match bot_token {
            None => {
                tracing::warn!("no bot_token for tenant_id={}, skipping", tenant_id);
                self.mark_processed(id).await;
                return;
            }
            Some(t) => t,
        };

        let chat_id = payload["chat_id"].as_i64();
        let message = format_message(event_type, payload);

        match (chat_id, message) {
            (Some(chat_id), Some(message)) => {
                self.telegram_client
                    .send_message(&bot_token, chat_id, &message)
                    .await;
            }
            _ => tracing::warn!("cannot format message for id={} type={}", id, event_type),
        }

        self.mark_processed(id).await;
    }

    async fn fetch_bot_token(&self, tenant_id: i64) -> Option<String> {
        sqlx::query("SELECT bot_token FROM tenant WHERE id = $1")
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await
            .ok()
            .flatten()
            .and_then(|row| row.try_get("bot_token").ok())
            .filter(|t: &String| !t.is_empty())
    }

    async fn mark_processed(&self, id: i64) {
        if let Err(e) =
            sqlx::query("UPDATE notification_outbox SET processed_at = now() WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
        {
            tracing::error!("failed to mark entry {} processed: {}", id, e);
        }
    }
}

fn format_message(event_type: &str, payload: &Value) -> Option<String> {
    match event_type {
        "BOOKING_CREATED" => {
            let booking_id = payload["booking_id"].as_i64()?;
            let customer_name = payload["customer_name"].as_str()?;
            let total_price = payload["total_price"].as_str()?;
            let currency = payload["currency"].as_str()?;
            let delivery_date = payload["delivery_date"].as_str()?;
            let delivery_address = payload["delivery_address"].as_str()?;

            let mut text = format!(
                "🆕 <b>New order #{}</b>\nCustomer: {}\nTotal: {} {}\nDelivery: {}\nAddress: {}",
                booking_id, customer_name, total_price, currency, delivery_date, delivery_address
            );
            if let Some(phone) = payload["customer_phone"].as_str().filter(|s| !s.is_empty()) {
                text.push_str(&format!("\nPhone: {}", phone));
            }
            if let Some(note) = payload["note"].as_str().filter(|s| !s.is_empty()) {
                text.push_str(&format!("\nNote: {}", note));
            }
            if let Some(items) = payload["items"].as_array().filter(|a| !a.is_empty()) {
                text.push_str("\nItems:");
                for item in items {
                    if let (Some(name), Some(qty)) =
                        (item["service_name"].as_str(), item["quantity"].as_i64())
                    {
                        text.push_str(&format!("\n• {} × {}", name, qty));
                    }
                }
            }
            Some(text)
        }
        "STATUS_CHANGED" => {
            let booking_id = payload["booking_id"].as_i64()?;
            let new_status = payload["new_status"].as_str()?;
            let delivery_date = payload["delivery_date"].as_str().unwrap_or("");
            let tracking_url = payload["tracking_url"].as_str().unwrap_or("");

            let text = match new_status {
                "CONFIRMED" => format!(
                    "✅ <b>Order #{} confirmed</b>\nDelivery: {}",
                    booking_id, delivery_date
                ),
                "DELIVERING" => {
                    let base = format!("🚗 <b>Order #{} is on the way</b>", booking_id);
                    if !tracking_url.is_empty() {
                        format!("{}\nTrack: {}", base, tracking_url)
                    } else {
                        base
                    }
                }
                "DONE" => format!("✅ <b>Order #{} delivered</b>\nThank you!", booking_id),
                "CANCELLED" => format!("❌ <b>Order #{} cancelled</b>", booking_id),
                _ => return None,
            };
            Some(text)
        }
        "PAYMENT_CONFIRMED" => {
            let booking_id = payload["booking_id"].as_i64()?;
            let customer_name = payload["customer_name"].as_str()?;
            let total_price = payload["total_price"].as_str()?;
            let currency = payload["currency"].as_str()?;
            Some(format!(
                "💰 <b>Payment confirmed</b>\nOrder #{}\nCustomer: {}\nTotal: {} {}",
                booking_id, customer_name, total_price, currency
            ))
        }
        _ => {
            tracing::warn!("unknown event_type: {}", event_type);
            None
        }
    }
}
