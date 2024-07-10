//it invokes every hour, drives the data gathering, db population, commenting on issues as notice task 

use chrono::{Timelike, Utc};
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_populate::get_pool;
use gosim_project::the_paced_runner::*;
use schedule_flows::{schedule_cron_job, schedule_handler};

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    let now = Utc::now();
    let now_minute = now.minute() + 2;
    let cron_time = format!("{:02} {:02} * * *", now_minute, now.hour());
    let cron_time = String::from("57 * * * *");
    schedule_cron_job(cron_time, String::from("cron_job_evoked")).await;
}

#[schedule_handler]
async fn handler(body: Vec<u8>) {
    dotenv().ok();
    let _ = inner(body).await;
}

pub async fn inner(_body: Vec<u8>) -> anyhow::Result<()> {
    logger::init();
    let pool = get_pool().await;
    let _ = run_hourly(&pool).await;

    Ok(())
}
