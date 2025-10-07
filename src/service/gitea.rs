use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct CreateIssueResponse {
    pub id: i64,
    pub url: String,
}

pub async fn create_issue(title: String, body: String, pool: &Pool) -> Result<CreateIssueResponse> {
    let conf = db::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "title": title, "body": body });
    let gitea_response = client
        .post("https://gitea.btcmap.org/api/v1/repos/teambtcmap/btcmap-data/issues")
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .json(&args)
        .send()
        .await?;
    if !gitea_response.status().is_success() {
        return Err("failed to create gitea issue".into());
    }
    let gitea_response: CreateIssueResponse = gitea_response.json().await?;
    Ok(gitea_response)
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct GetIssueResponse {
    pub id: i64,
    pub state: String,
}

pub async fn get_issue(issue_url: String, pool: &Pool) -> Result<GetIssueResponse> {
    let conf = db::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let gitea_response = client
        .get(issue_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .send()
        .await?;
    if !gitea_response.status().is_success() {
        return Err("failed to fetch gitea issue".into());
    }
    let gitea_response: GetIssueResponse = gitea_response.json().await?;
    Ok(gitea_response)
}
