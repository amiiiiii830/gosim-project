use chrono::{Duration, Utc};
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
// use github_flows::{get_octo, GithubLogin};
use crate::issue_tracker::*;

/* pub async fn comment_on_issue(
    project_id: &str,
    issue_id: &str,
    comment: &str,
) -> anyhow::Result<()> {
    dotenv().ok();
    // let issue_id = "https://github.com/alabulei1/a-test/issues/87";
    // let project_id = "https://github.com/KwickerHub/WebCraftifyAI";
    let parts: Vec<&str> = project_id.rsplitn(3, '/').collect();
    let (repo, owner) = (parts[0].to_string(), parts[1].to_string());

    let issue_parts: Vec<&str> = issue_id.rsplitn(2, '/').collect();
    let issue_number = issue_parts[0].parse::<i32>().unwrap_or(0);
    let (owner, repo) = ("alabulei1", "a-test");
    let issue_number = 87;

    let octocrab = get_octo(&GithubLogin::Default);

    let report_issue_handle = octocrab.issues(owner, repo);

    let n_days_ago = (Utc::now() - Duration::hours(1)).naive_utc();

    match report_issue_handle
        .create_comment(issue_number, comment)
        .await
    {
        Ok(_) => (),
        Err(e) => log::error!("Error commenting on issue: {:?}", e),
    }
    Ok(())
} */

pub async fn mock_comment_on_issue(issue_id: &str, comment: &str) -> anyhow::Result<()> {
    // let issue_id = "https://github.com/alabulei1/a-test/issues/87";
    // let project_id = "https://github.com/KwickerHub/WebCraftifyAI";
    let (owner, repo) = ("alabulei1", "a-test");
    let issue_number = 87;

    let url = format!("https://api.github.com/repos/{owner}/{repo}/issues/{issue_number}/comments");

    match github_http_post(&url, comment).await {
        Ok(_) => (),

        Err(e) => log::error!("Error commenting on issue: {:?}", e),
    }

    // let octocrab = get_octo(&GithubLogin::Default);
    // let report_issue_handle = octocrab.issues(owner, repo);

    // match report_issue_handle
    //     .create_comment(issue_number, comment)
    //     .await
    // {
    //     Ok(_) => (),
    //     Err(e) => log::error!("Error commenting on issue: {:?}", e),
    // }

    Ok(())
}
