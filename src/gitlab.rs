use eyre::{Result, WrapErr};
use serde::Deserialize;

use gitlab::api::{self, AsyncQuery, Pagination};
use gitlab::Gitlab;

use crate::config::Config;
use crate::parser::Dashboard;

#[derive(Debug, Deserialize)]
struct GitlabProject {
    id: u64,
    #[serde(default)]
    path_with_namespace: String,
    #[serde(default)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct GitlabIssue {
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    web_url: String,
}

pub struct GitlabClient {
    client: gitlab::AsyncGitlab,
    base_url: String,
}

impl GitlabClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let parsed = url::Url::parse(&config.gitlab_url)
            .wrap_err("Invalid GITLAB_URL")?;
        let host = parsed.host_str()
            .ok_or_else(|| eyre::eyre!("GITLAB_URL missing host"))?;
        let client = Gitlab::builder(host, &config.gitlab_token)
            .build_async()
            .await
            .wrap_err("Failed to create GitLab client")?;
        Ok(GitlabClient {
            client,
            base_url: config.gitlab_url.trim_end_matches('/').to_string(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn pr_search_url(&self, full_name: &str, branch: &str) -> String {
        format!("{}/{}/-/merge_requests?search={}", self.base_url, full_name, branch)
    }

    pub async fn fetch_dashboards(&self) -> Result<Vec<(String, String, String, Dashboard)>> {
        let mut dashboards = Vec::new();

        let endpoint = gitlab::api::projects::Projects::builder()
            .build()
            .wrap_err("Failed to build projects query")?;

        let projects: Vec<GitlabProject> = api::paged(endpoint, Pagination::All)
            .query_async(&self.client)
            .await
            .wrap_err("Failed to list GitLab projects")?;

        for project in &projects {
            let full_name = &project.path_with_namespace;
            let repo_name = &project.name;

            match self.find_dashboard(project.id, full_name).await {
                Ok(Some((dashboard, dashboard_url))) => {
                    dashboards.push((full_name.clone(), repo_name.clone(), dashboard_url, dashboard));
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("Warning: Failed to fetch dashboard for {}: {}", full_name, e);
                }
            }
        }

        Ok(dashboards)
    }

    async fn find_dashboard(&self, project_id: u64, full_name: &str) -> Result<Option<(Dashboard, String)>> {
        let endpoint = gitlab::api::projects::issues::Issues::builder()
            .project(project_id)
            .state(gitlab::api::projects::issues::IssueState::Opened)
            .search("Dependency Dashboard")
            .build()
            .wrap_err_with(|| format!("Failed to build issues query for {}", full_name))?;

        let issues: Vec<GitlabIssue> = api::paged(endpoint, Pagination::All)
            .query_async(&self.client)
            .await
            .wrap_err_with(|| format!("Failed to list issues for {}", full_name))?;

        for issue in &issues {
            if issue.title == "Dependency Dashboard" {
                let dashboard = crate::parser::parse_dashboard(&issue.description)?;
                return Ok(Some((dashboard, issue.web_url.clone())));
            }
        }

        Ok(None)
    }
}
