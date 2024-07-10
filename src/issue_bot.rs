//function that post comments to issues

use crate::issue_paced_tracker::*;

pub async fn comment_on_issue(issue_id: &str, comment: &str) -> anyhow::Result<()> {
    // let issue_id = "https://github.com/alabulei1/a-test/issues/87";
    let issue_parts: Vec<&str> = issue_id.rsplitn(5, '/').collect();
    let issue_number = issue_parts[0].parse::<i32>().unwrap_or(0);
    let (repo, owner) = (issue_parts[2].to_string(), issue_parts[3].to_string());

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}/comments");

    if let Err(e) = github_http_post(&url, comment).await {
        log::error!("Error commenting on issue: {:?}", e);
    }
    Ok(())
}

