use eyre::Result;

use crate::forgejo::ForgejoClient;
use crate::gitlab::GitlabClient;
use crate::bitbucket::BitbucketClient;
use crate::parser::Dashboard;

pub enum PlatformClient {
    Forgejo(ForgejoClient),
    Gitlab(GitlabClient),
    Bitbucket(BitbucketClient),
}

impl PlatformClient {
    pub async fn fetch_dashboards(&self) -> Result<Vec<(String, String, String, Dashboard)>> {
        match self {
            PlatformClient::Forgejo(c) => c.fetch_dashboards().await,
            PlatformClient::Gitlab(c) => c.fetch_dashboards().await,
            PlatformClient::Bitbucket(c) => c.fetch_dashboards().await,
        }
    }

    pub fn pr_search_url(&self, full_name: &str, branch: &str) -> String {
        match self {
            PlatformClient::Forgejo(c) => c.pr_search_url(full_name, branch),
            PlatformClient::Gitlab(c) => c.pr_search_url(full_name, branch),
            PlatformClient::Bitbucket(c) => c.pr_search_url(full_name, branch),
        }
    }

    pub fn base_url(&self) -> &str {
        match self {
            PlatformClient::Forgejo(c) => c.base_url(),
            PlatformClient::Gitlab(c) => c.base_url(),
            PlatformClient::Bitbucket(c) => c.base_url(),
        }
    }


}
