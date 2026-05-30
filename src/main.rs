mod settings;
mod processors;
mod telegram;

use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("yoobu_notifier=debug".parse().unwrap()),
        )
        .init();

    let config = settings::Settings::from_env();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    tracing::info!("connected to database");

    let telegram = telegram::TelegramClient::new();
    let processor = processors::Processor::new(pool, telegram);

    tracing::info!("yoobu-notifier started");

    processor.run().await;
}
