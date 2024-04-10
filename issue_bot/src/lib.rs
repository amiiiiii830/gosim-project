use chrono::{Datelike, Duration, Timelike, Utc};
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use github_flows::{get_octo, GithubLogin};
use octocrab_wasi::params::State;
use octocrab_wasi::{params::issues::Sort, params::Direction};
use schedule_flows::{schedule_cron_job, schedule_handler};
use std::{
    collections::{HashMap, HashSet},
    env,
};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    // let now = Utc::now();
    // let now_minute = now.minute() + 2;
    // let cron_time = format!("{:02} {:02} {:02} * *", now_minute, now.hour(), now.day());
    let cron_time = format!("2 * * * *");
    schedule_cron_job(cron_time, String::from("cron_job_evoked")).await;
}

#[schedule_handler]
async fn handler(body: Vec<u8>) {
    dotenv().ok();
    logger::init();

    let _ = inner().await;
}
async fn inner() -> anyhow::Result<()> {
    let octocrab = get_octo(&GithubLogin::Default);

    let report_issue_handle = octocrab.issues("jaykchen", "issue-labeler");

    let n_days_ago = (Utc::now() - Duration::hours(1)).naive_utc();

    let report_issue = report_issue_handle
        .create_comment(1329, "hard coded demo")
        .await?;
    Ok(())
}
