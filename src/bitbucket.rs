use eyre::{Result, WrapErr};
use serde::Deserialize;

use crate::config::Config;
use crate::parser::Dashboard;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
struct BitbucketPrsPage {
    values: Vec<BitbucketPr>,
    #[serde(default)]
    is_last_page: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitbucketPr {
    id: u64,
    title: String,
    from_ref: FromRef,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FromRef {
    display_id: String,
    latest_commit: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitbucketBuildStatusPage {
    values: Vec<BitbucketBuildStatus>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BitbucketBuildStatus {
    state: String,
}

pub struct BitbucketClient {
    base_url: String,
    token: String,
    client: reqwest::Client,
    project: Option<String>,
}

impl BitbucketClient {
    pub fn new(config: &Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .wrap_err("Failed to build HTTP client")?;
        Ok(BitbucketClient {
            base_url: config.bitbucket_url.trim_end_matches('/').to_string(),
            token: config.bitbucket_token.clone(),
            client,
            project: config.bitbucket_project.clone(),
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
            "{}/projects/{}/repos/{}/pull-requests?q={}",
            self.base_url, project_key, repo_slug, branch
        )
    }

    pub async fn fetch_dashboards(&self) -> Result<Vec<(String, String, String, Dashboard)>> {
        let mut dashboards = Vec::new();

        if let Some(ref project_key) = self.project {
            let mut repo_start = 0;
            loop {
                let repos = self.list_repos(project_key, repo_start).await?;

                if repos.values.is_empty() {
                    break;
                }

                for repo in &repos.values {
                    let full_name = format!("{}/{}", project_key, repo.slug);

                    match self.find_renovate_prs(project_key, &repo.slug, &full_name).await {
                        Ok((dashboard, dashboard_url)) => {
                            dashboards.push((full_name, repo.name.clone(), dashboard_url, dashboard));
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to fetch PRs for {}: {}", full_name, e);
                        }
                    }
                }

                if repos.is_last_page {
                    break;
                }
                repo_start += repos.values.len() as u32;
            }
        } else {
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

                            match self.find_renovate_prs(&project.key, &repo.slug, &full_name).await {
                                Ok((dashboard, dashboard_url)) => {
                                    dashboards.push((full_name, repo.name.clone(), dashboard_url, dashboard));
                                }
                                Err(e) => {
                                    eprintln!("Warning: Failed to fetch PRs for {}: {}", full_name, e);
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

    async fn find_renovate_prs(
        &self,
        project_key: &str,
        repo_slug: &str,
        full_name: &str,
    ) -> Result<(Dashboard, String)> {
        let dashboard_url = format!(
            "{}/projects/{}/repos/{}/pull-requests",
            self.base_url, project_key, repo_slug
        );

        let mut dashboard = Dashboard {
            pending_approval: Vec::new(),
            open: Vec::new(),
            awaiting_schedule: Vec::new(),
            rate_limited: Vec::new(),
            errored: Vec::new(),
            pending_automerge: Vec::new(),
            other: Vec::new(),
        };

        let mut start = 0;
        loop {
            let url = format!(
                "{}/rest/api/1.0/projects/{}/repos/{}/pull-requests?state=OPEN&limit=50&start={}",
                self.base_url, project_key, repo_slug, start
            );

            let response = self.client
                .get(&url)
                .bearer_auth(&self.token)
                .send()
                .await
                .wrap_err_with(|| format!("Failed to list PRs for {}", full_name))?;

            if response.status() == reqwest::StatusCode::NOT_FOUND {
                return Ok((dashboard, dashboard_url));
            }

            let page = response
                .error_for_status()
                .wrap_err_with(|| format!("Bitbucket API error listing PRs for {}", full_name))?
                .json::<BitbucketPrsPage>()
                .await
                .wrap_err_with(|| format!("Failed to parse PRs response for {}", full_name))?;

            if page.values.is_empty() {
                break;
            }

            for pr in &page.values {
                let branch = &pr.from_ref.display_id;
                if !branch.starts_with("renovate/") {
                    continue;
                }
                let build_status = self.get_build_status(&pr.from_ref.latest_commit).await;
                let update = crate::parser::Update {
                    branch: branch.clone(),
                    description: format!("[PR #{}] {}", pr.id, pr.title),
                    checked: false,
                    build_status: Some(build_status),
                };
                dashboard.open.push(update);
            }

            if page.is_last_page {
                break;
            }
            start += page.values.len() as u32;
        }

        Ok((dashboard, dashboard_url))
    }

    async fn get_build_status(&self, commit: &str) -> String {
        let url = format!(
            "{}/rest/build-status/1.0/commits/{}",
            self.base_url, commit
        );
        let page = match self.client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
        {
            Ok(resp) => match resp.json::<BitbucketBuildStatusPage>().await {
                Ok(p) => p,
                Err(_) => return "unknown".to_string(),
            },
            Err(_) => return "unknown".to_string(),
        };

        if page.values.is_empty() {
            return "no_build".to_string();
        }

        if page.values.iter().any(|s| s.state == "FAILED") {
            "failed".to_string()
        } else if page.values.iter().any(|s| s.state == "INPROGRESS") {
            "in_progress".to_string()
        } else if page.values.iter().all(|s| s.state == "SUCCESSFUL") {
            "successful".to_string()
        } else {
            "unknown".to_string()
        }
    }
}
