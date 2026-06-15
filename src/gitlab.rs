use eyre::{Result, WrapErr};
use serde::Deserialize;

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
    #[allow(dead_code)]
    iid: u64,
}

pub struct GitlabClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl GitlabClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .wrap_err("Failed to build HTTP client")?;
        Ok(GitlabClient {
            base_url: config.gitlab_url.clone(),
            token: config.gitlab_token.clone(),
            client,
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
        let mut page = 1;

        loop {
            let projects = self.list_projects(page).await?;

            if projects.is_empty() {
                break;
            }

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

            page += 1;
        }

        Ok(dashboards)
    }

    async fn list_projects(&self, page: u32) -> Result<Vec<GitlabProject>> {
        let url = format!("{}/api/v4/projects?per_page=50&page={}", self.base_url, page);
        let projects = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .wrap_err_with(|| format!("Failed to list GitLab projects (page {})", page))?
            .error_for_status()
            .wrap_err_with(|| format!("GitLab API error listing projects (page {})", page))?
            .json::<Vec<GitlabProject>>()
            .await
            .wrap_err_with(|| format!("Failed to parse GitLab projects response (page {})", page))?;
        Ok(projects)
    }

    async fn find_dashboard(&self, project_id: u64, full_name: &str) -> Result<Option<(Dashboard, String)>> {
        let url = format!(
            "{}/api/v4/projects/{}/issues?state=opened&search=Dependency+Dashboard&per_page=100",
            self.base_url, project_id
        );
        let issues = self.client
            .get(&url)
            .header("PRIVATE-TOKEN", &self.token)
            .send()
            .await
            .wrap_err_with(|| format!("Failed to list issues for {}", full_name))?
            .error_for_status()
            .wrap_err_with(|| format!("GitLab API error listing issues for {}", full_name))?
            .json::<Vec<GitlabIssue>>()
            .await
            .wrap_err_with(|| format!("Failed to parse issues response for {}", full_name))?;

        for issue in &issues {
            if issue.title == "Dependency Dashboard" {
                let dashboard = crate::parser::parse_dashboard(&issue.description)?;
                return Ok(Some((dashboard, issue.web_url.clone())));
            }
        }

        Ok(None)
    }
}
