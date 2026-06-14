mod config;
mod forgejo;
mod parser;
mod report;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let config = config::Config::from_env()?;

    println!("Connecting to {}...", config.forgejo_url);

    let client = forgejo::ForgejoClient::new(&config)?;

    println!("Fetching repositories and dashboards...");
    let dashboards = client.fetch_dashboards().await?;

    println!("Found {} dashboard(s)", dashboards.len());

    println!("Generating report...");
    report::generate_report(&dashboards, &config.forgejo_url, &config.report_output)?;

    println!("Report saved to {}", config.report_output);

    Ok(())
}
