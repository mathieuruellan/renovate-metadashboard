use std::env;
use eyre::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Platform {
    Forgejo,
    Gitlab,
    Bitbucket,
}

impl Platform {
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "forgejo" => Ok(Platform::Forgejo),
            "gitlab" => Ok(Platform::Gitlab),
            "bitbucket" => Ok(Platform::Bitbucket),
            _ => Err(eyre::eyre!("Unknown platform: {}. Supported: forgejo, gitlab, bitbucket", s)),
        }
    }
}

pub struct Config {
    pub platform: Platform,
    pub forgejo_url: String,
    pub forgejo_token: String,
    pub gitlab_url: String,
    pub gitlab_token: String,
    pub bitbucket_url: String,
    pub bitbucket_token: String,
    pub bitbucket_project: Option<String>,
    pub report_output: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        let platform_str = env::var("PLATFORM").unwrap_or_else(|_| "forgejo".to_string());
        let platform = Platform::from_str(&platform_str)?;

        let forgejo_url = env::var("FORGEJO_URL").unwrap_or_default();
        let forgejo_token = env::var("FORGEJO_TOKEN").unwrap_or_default();
        let gitlab_url = env::var("GITLAB_URL").unwrap_or_default();
        let gitlab_token = env::var("GITLAB_TOKEN").unwrap_or_default();
        let bitbucket_url = env::var("BITBUCKET_URL").unwrap_or_default();
        let bitbucket_token = env::var("BITBUCKET_TOKEN").unwrap_or_default();
        let bitbucket_project = env::var("BITBUCKET_PROJECT").ok().filter(|s| !s.is_empty());

        let report_output = env::var("REPORT_OUTPUT_FILE")
            .unwrap_or_else(|_| "report.html".to_string());

        match platform {
            Platform::Forgejo => {
                if forgejo_url.is_empty() {
                    return Err(eyre::eyre!("FORGEJO_URL not set"));
                }
                if forgejo_token.is_empty() {
                    return Err(eyre::eyre!("FORGEJO_TOKEN not set"));
                }
            }
            Platform::Gitlab => {
                if gitlab_url.is_empty() {
                    return Err(eyre::eyre!("GITLAB_URL not set"));
                }
                if gitlab_token.is_empty() {
                    return Err(eyre::eyre!("GITLAB_TOKEN not set"));
                }
            }
            Platform::Bitbucket => {
                if bitbucket_url.is_empty() {
                    return Err(eyre::eyre!("BITBUCKET_URL not set"));
                }
                if bitbucket_token.is_empty() {
                    return Err(eyre::eyre!("BITBUCKET_TOKEN not set"));
                }
            }
        }

        Ok(Config {
            platform,
            forgejo_url,
            forgejo_token,
            gitlab_url,
            gitlab_token,
            bitbucket_url,
            bitbucket_token,
            bitbucket_project,
            report_output,
        })
    }


}
