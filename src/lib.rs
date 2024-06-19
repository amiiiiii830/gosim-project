pub mod db_join;
pub mod db_manipulate;
pub mod db_populate;
pub mod issue_bot;
pub mod issue_paced_tracker;
pub mod llm_utils;
pub mod llm_utils_together;
pub mod the_paced_runner;
pub mod vector_search;
use chrono::{Duration, Timelike, Utc};
use lazy_static::lazy_static;

pub static TOTAL_BUDGET: i32 = 50_000;
pub static ISSUE_LABEL: &str = "gosim-bounty";
pub static PR_LABEL: &str = "gosim-bounty-accepted";
pub static START_DATE: &str = "2024-06-17";
pub static END_DATE: &str = "2024-07-17";

lazy_static! {
    pub static ref THIS_HOUR: String = {
        // let date = Utc::now().date_naive();
        let datetime = Utc::now().with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .expect("Invalid time");
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };
    pub static ref NEXT_HOUR: String = {
        // let date = Utc::now().date_naive();
        let datetime = Utc::now().with_minute(0)
        .and_then(|dt| dt.with_second(0))
        .expect("Invalid time");
   let previous_hour= datetime + Duration::hours(1);

    previous_hour.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };
    pub static ref TODAY_THIS_HOUR: u32 = Utc::now().hour();
}
