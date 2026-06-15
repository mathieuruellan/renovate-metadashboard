use eyre::{Result, WrapErr};
use serde::Deserialize;

use crate::config::Config;
use crate::parser::Dashboard;

#[derive(Debug, Deserialize)]
struct BitbucketProjectsPage {
    values: Vec<BitbucketProject>,
    #[serde(default)]
    is_last_page: bool,
}

#[derive(Debug, Deserialize)]
struct BitbucketProject {
    key: String,
    #[allow(dead_code)]
    name: String,
}

#[derive(Debug, Deserialize)]
struct BitbucketReposPage {
    values: Vec<BitbucketRepo>,
    #[serde(default)]
    is_last_page: bool,
}

#[derive(Debug, Deserialize)]
struct BitbucketRepo {
    slug: String,
    name: String,
    #[allow(dead_code)]
    project: Option<BitbucketProjectRef>,
}

#[derive(Debug, Deserialize)]
struct BitbucketProjectRef {
    #[allow(dead_code)]
    key: String,
}

#[derive(Debug, Deserialize)]
struct BitbucketIssuesPage {
    values: Vec<BitbucketIssue>,
    #[serde(default)]
    #[allow(dead_code)]
    is_last_page: bool,
}

#[derive(Debug, Deserialize)]
struct BitbucketIssue {
    id: u64,
    title: String,
    content: Option<String>,
}

pub struct BitbucketClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
}

impl BitbucketClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .wrap_err("Failed to build HTTP client")?;
        Ok(BitbucketClient {
            base_url: config.bitbucket_url.clone(),
            token: config.bitbucket_token.clone(),
            client,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn pr_search_url(&self, full_name: &str, branch: &str) -> String {
        let parts: Vec<&str> = full_name.splitn(2, '/').collect();
        let (project_key, repo_slug) = match parts.as_slice() {
            [key, slug] => (*key, *slug),
            _ => ("", ""),
        };
        format!(
            "{}/projects/{}/repos/{}/pull-requests?at=refs/heads/{}",
            self.base_url, project_key, repo_slug, branch
        )
    }

    pub async fn fetch_dashboards(&self) -> Result<Vec<(String, String, String, Dashboard)>> {
        let mut dashboards = Vec::new();
        let mut project_start = 0;

        loop {
            let projects = self.list_projects(project_start).await?;

            if projects.values.is_empty() {
                break;
            }

            for project in &projects.values {
                let mut repo_start = 0;
                loop {
                    let repos = self.list_repos(&project.key, repo_start).await?;

                    if repos.values.is_empty() {
                        break;
                    }

                    for repo in &repos.values {
                        let full_name = format!("{}/{}", project.key, repo.slug);

                        match self.find_dashboard(&project.key, &repo.slug, &full_name).await {
                            Ok(Some((dashboard, dashboard_url))) => {
                                dashboards.push((full_name, repo.name.clone(), dashboard_url, dashboard));
                            }
                            Ok(None) => {}
                            Err(e) => {
                                eprintln!("Warning: Failed to fetch dashboard for {}: {}", full_name, e);
                            }
                        }
                    }

                    if repos.is_last_page {
                        break;
                    }
                    repo_start += repos.values.len() as u32;
                }
            }

            if projects.is_last_page {
                break;
            }
            project_start += projects.values.len() as u32;
        }

        Ok(dashboards)
    }

    async fn list_projects(&self, start: u32) -> Result<BitbucketProjectsPage> {
        let url = format!("{}/rest/api/1.0/projects?limit=50&start={}", self.base_url, start);
        let page = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .wrap_err_with(|| format!("Failed to list Bitbucket projects (start={})", start))?
            .error_for_status()
            .wrap_err_with(|| format!("Bitbucket API error listing projects (start={})", start))?
            .json::<BitbucketProjectsPage>()
            .await
            .wrap_err_with(|| format!("Failed to parse Bitbucket projects response (start={})", start))?;
        Ok(page)
    }

    async fn list_repos(&self, project_key: &str, start: u32) -> Result<BitbucketReposPage> {
        let url = format!(
            "{}/rest/api/1.0/projects/{}/repos?limit=50&start={}",
            self.base_url, project_key, start
        );
        let page = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .wrap_err_with(|| format!("Failed to list repos for project {}", project_key))?
            .error_for_status()
            .wrap_err_with(|| format!("Bitbucket API error listing repos for project {}", project_key))?
            .json::<BitbucketReposPage>()
            .await
            .wrap_err_with(|| format!("Failed to parse repos response for project {}", project_key))?;
        Ok(page)
    }

    async fn find_dashboard(
        &self,
        project_key: &str,
        repo_slug: &str,
        full_name: &str,
    ) -> Result<Option<(Dashboard, String)>> {
        let url = format!(
            "{}/rest/api/1.0/projects/{}/repos/{}/issues?limit=50",
            self.base_url, project_key, repo_slug
        );
        let page = self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .wrap_err_with(|| format!("Failed to list issues for {}", full_name))?
            .error_for_status()
            .wrap_err_with(|| format!("Bitbucket API error listing issues for {}", full_name))?
            .json::<BitbucketIssuesPage>()
            .await
            .wrap_err_with(|| format!("Failed to parse issues response for {}", full_name))?;

        for issue in &page.values {
            if issue.title == "Dependency Dashboard" {
                let body = issue.content.clone().unwrap_or_default();
                let dashboard = crate::parser::parse_dashboard(&body)?;
                let dashboard_url = format!(
                    "{}/projects/{}/repos/{}/issues/{}",
                    self.base_url, project_key, repo_slug, issue.id
                );
                return Ok(Some((dashboard, dashboard_url)));
            }
        }

        Ok(None)
    }
}
