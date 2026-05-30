pub struct Config {
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        let db_url = std::env::var("DB_URL").expect("DB_URL must be set");
        let user = std::env::var("DB_USER").expect("DB_USER must be set");
        let pass = std::env::var("DB_PASS").expect("DB_PASS must be set");

        // Strip jdbc: prefix — SQLx uses postgresql:// directly
        let base = db_url.strip_prefix("jdbc:").unwrap_or(&db_url);
        let database_url = base.replacen("://", &format!("://{user}:{pass}@"), 1);

        Self { database_url }
    }
}
