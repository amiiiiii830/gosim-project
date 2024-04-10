use chrono::{Datelike, Duration, Timelike, Utc};
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{get_octo, GithubLogin};
use std::{
    collections::{HashMap, HashSet},
    env,
};

async fn comment_on_issue(project_id: &str, issue_id: &str, comment: &str) -> anyhow::Result<()> {
    dotenv().ok();
    logger::init();
    // let issue_id = "https://github.com/KwickerHub/WebCraftifyAI/issues/2798";
    // let project_id = "https://github.com/KwickerHub/WebCraftifyAI";

    let parts: Vec<&str> = project_id.rsplitn(3, '/').collect();
    let (repo, owner) = (parts[0].to_string(), parts[1].to_string());

    let issue_parts: Vec<&str> = issue_id.rsplitn(2, '/').collect();
    let issue_number = issue_parts[0].parse::<i32>().unwrap_or(0);

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
}
