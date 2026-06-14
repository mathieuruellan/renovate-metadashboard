use std::env;
use eyre::{Result, WrapErr};

pub struct Config {
    pub forgejo_url: String,
    pub forgejo_token: String,
    pub report_output: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let forgejo_url = env::var("FORGEJO_URL")
            .wrap_err("FORGEJO_URL not set in environment or .env")?;
        let forgejo_token = env::var("FORGEJO_TOKEN")
            .wrap_err("FORGEJO_TOKEN not set in environment or .env")?;
        let report_output = env::var("REPORT_OUTPUT_FILE")
            .unwrap_or_else(|_| "report.html".to_string());

        Ok(Config {
            forgejo_url,
            forgejo_token,
            report_output,
        })
    }
}
