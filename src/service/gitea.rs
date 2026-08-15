use crate::{db, Result};
use deadpool_sqlite::Pool;
use serde::Deserialize;
use serde_json::json;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct CreateIssueResponse {
    pub id: i64,
    pub url: String,
    pub html_url: String,
}

pub async fn create_issue(
    title: String,
    body: String,
    labels: Vec<i64>,
    pool: &Pool,
) -> Result<CreateIssueResponse> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "title": title, "body": body, "labels": labels });
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
pub struct GiteaLabel {
    pub id: i64,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct GetIssueResponse {
    pub id: i64,
    pub state: String,
    pub html_url: String,
    pub labels: Vec<GiteaLabel>,
}

pub async fn get_issue(issue_url: String, pool: &Pool) -> Result<Option<GetIssueResponse>> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let gitea_response = client
        .get(issue_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .send()
        .await?;
    if gitea_response.status().as_u16() == 404 {
        return Ok(None);
    }
    if !gitea_response.status().is_success() {
        return Err("failed to fetch gitea issue".into());
    }
    let gitea_response: GetIssueResponse = gitea_response.json().await?;
    Ok(Some(gitea_response))
}

pub async fn close_issue(issue_url: &str, pool: &Pool) -> Result<()> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "state": "closed" });
    let response = client
        .patch(issue_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .json(&args)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err("failed to close gitea issue".into());
    }
    Ok(())
}

pub async fn reopen_issue(issue_url: &str, pool: &Pool) -> Result<()> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "state": "open" });
    let response = client
        .patch(issue_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .json(&args)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err("failed to reopen gitea issue".into());
    }
    Ok(())
}

pub async fn set_issue_labels(issue_url: &str, labels: Vec<i64>, pool: &Pool) -> Result<()> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "labels": labels });
    let labels_url = format!("{}/labels", issue_url);
    let response = client
        .put(&labels_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .json(&args)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err("failed to set gitea issue labels".into());
    }
    Ok(())
}

pub async fn add_issue_comment(issue_url: &str, body: &str, pool: &Pool) -> Result<()> {
    let conf = db::main::conf::queries::select(pool).await?;
    if conf.gitea_api_key.is_empty() {
        Err("gitea api key is not set")?
    }
    let client = reqwest::Client::new();
    let args = json!({ "body": body });
    let comments_url = format!("{}/comments", issue_url);
    let response = client
        .post(&comments_url)
        .header("Authorization", format!("token {}", conf.gitea_api_key))
        .json(&args)
        .send()
        .await?;
    if !response.status().is_success() {
        return Err("failed to add gitea issue comment".into());
    }
    Ok(())
}
