use eyre::{Result, WrapErr};
use forgejo_api::structs::{IssueListIssuesQuery, IssueListIssuesQueryState, RepoSearchQuery};
use forgejo_api::{Auth, Forgejo};

use crate::config::Config;
use crate::parser::Dashboard;

pub struct ForgejoClient {
    api: Forgejo,
}

impl ForgejoClient {
    pub fn new(config: &Config) -> Result<Self> {
        let url = url::Url::parse(&config.forgejo_url)
            .wrap_err_with(|| format!("Invalid FORGEJO_URL: {}", config.forgejo_url))?;
        let api = Forgejo::new(Auth::Token(&config.forgejo_token), url)?;
        Ok(ForgejoClient { api })
    }

    pub async fn fetch_dashboards(&self) -> Result<Vec<(String, String, String, Dashboard)>> {
        let mut dashboards = Vec::new();
        let mut page = 1;

        loop {
            let results = self
                .api
                .repo_search(RepoSearchQuery {
                    q: Some("".to_string()),
                    ..Default::default()
                })
                .page(page)
                .page_size(50)
                .send()
                .await
                .wrap_err("Failed to search repositories")?;

            let repos = match results.data {
                Some(r) => r,
                None => break,
            };

            if repos.is_empty() {
                break;
            }

            for repo in &repos {
                let owner = repo.owner.as_ref().map(|o| o.login.as_deref()).flatten().unwrap_or("unknown");
                let repo_name = repo.name.as_deref().unwrap_or("unknown");

                match self.find_dashboard(owner, repo_name).await {
                    Ok(Some((dashboard, dashboard_url))) => {
                        let full_name = format!("{}/{}", owner, repo_name);
                        dashboards.push((full_name, repo_name.to_string(), dashboard_url, dashboard));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        eprintln!("Warning: Failed to fetch dashboard for {}/{}: {}", owner, repo_name, e);
                    }
                }
            }

            page += 1;
        }

        Ok(dashboards)
    }

    async fn find_dashboard(&self, owner: &str, repo: &str) -> Result<Option<(Dashboard, String)>> {
        let issues = self
            .api
            .issue_list_issues(
                owner,
                repo,
                IssueListIssuesQuery {
                    state: Some(IssueListIssuesQueryState::Open),
                    ..Default::default()
                },
            )
            .all()
            .await
            .wrap_err_with(|| format!("Failed to list issues for {}/{}", owner, repo))?;

        for issue in &issues {
            if issue.title.as_deref() == Some("Dependency Dashboard") {
                let body = issue.body.clone().unwrap_or_default();
                let dashboard_url = issue.html_url.as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default();
                let dashboard = crate::parser::parse_dashboard(&body)?;
                return Ok(Some((dashboard, dashboard_url)));
            }
        }

        Ok(None)
    }
}
