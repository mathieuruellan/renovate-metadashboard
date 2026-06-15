mod client;
mod config;
mod forgejo;
mod gitlab;
mod bitbucket;
mod parser;
mod report;

use eyre::Result;

use crate::client::PlatformClient;
use crate::config::{Config, Platform};

fn build_client(config: &Config) -> Result<PlatformClient> {
    match config.platform {
        Platform::Forgejo => {
            Ok(PlatformClient::Forgejo(forgejo::ForgejoClient::new(config)?))
        }
        Platform::Gitlab => {
            Ok(PlatformClient::Gitlab(gitlab::GitlabClient::new(config)?))
        }
        Platform::Bitbucket => {
            Ok(PlatformClient::Bitbucket(bitbucket::BitbucketClient::new(config)?))
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    let client = build_client(&config)?;

    println!("Connecting to {}...", client.base_url());

    println!("Fetching repositories and dashboards...");
    let dashboards = client.fetch_dashboards().await?;

    println!("Found {} dashboard(s)", dashboards.len());

    println!("Generating report...");
    report::generate_report(&dashboards, &client, &config.report_output)?;

    println!("Report saved to {}", config.report_output);

    Ok(())
}
