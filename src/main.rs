pub mod config;
pub mod telegram;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("yoobu_notifier=debug".parse().unwrap()),
        )
        .init();

    let _config = config::Config::from_env();

    tracing::info!("yoobu-notifier starting...");
}
