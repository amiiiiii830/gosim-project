use chrono::{Timelike, Utc};
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_populate::get_pool;
use gosim_project::the_runner::*;
use schedule_flows::{schedule_cron_job, schedule_handler};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let now = Utc::now();
    let now_minute = now.minute() + 2;
    let cron_time = format!("{:02} {:02} * * *", now_minute, now.hour());
    let cron_time = String::from("38 * * * *");
    schedule_cron_job(cron_time, String::from("cron_job_evoked")).await;
}

#[schedule_handler]
async fn handler(body: Vec<u8>) {
    dotenv().ok();
    let _ = inner(body).await;
}

pub async fn inner(_body: Vec<u8>) -> anyhow::Result<()> {
    // let query = "repo:SarthakKeshari/calc_for_everything is:pr is:merged label:hacktoberfest-accepted created:2023-10-01..2023-10-03 review:approved -label:spam -label:invalid";
    // let query = "label:hacktoberfest is:issue is:open no:assignee created:2023-10-01..2023-10-03 -label:spam -label:invalid";

    // db_updater::test_add_project().await;
    // db_updater::test_project_exists().await;

    // let issues = search_issues_open(&query).await?;
    // let query = "repo:SarthakKeshari/calc_for_everything is:pr is:merged label:hacktoberfest-accepted created:2023-10-01..2023-10-30 review:approved -label:spam -label:invalid";
    // let pulls = get_per_repo_pull_requests(&query).await?;

    // let mut count = 0;
    // for iss in pulls {
    //     count += 1;
    //     log::error!("pull: {:?}", iss);
    //     let content = format!("{:?}", iss);
    //     // let _ = upload_to_gist(&content).await?;
    //     if count > 5 {
    //         break;
    //     }
    // }

    logger::init();
    let pool = get_pool().await;
    let _ = run_hourly(&pool).await;

    Ok(())
}

pub async fn search_pulls() -> anyhow::Result<()> {
    // let _ = upload_to_gist(&texts).await?;
    Ok(())
}
