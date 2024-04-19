pub mod db_join;
pub mod db_manipulate;
pub mod db_populate;
pub mod issue_bot;
pub mod issue_tracker;
pub mod the_runner;
pub mod vector_search;
pub mod llm_utils;
use chrono::{NaiveDate, Timelike, Utc};
use lazy_static::lazy_static;

pub static ISSUE_LABEL: &str = "hacktoberfest";
pub static PR_LABEL: &str = "hacktoberfest-accepted";
pub static START_DATE: &str = "2023-10-01";
pub static END_DATE: &str = "2023-10-30";

lazy_static! {
    pub static ref THIS_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-09", "%Y-%m-%d").unwrap();
        let datetime = date
            .and_hms_opt(Utc::now().hour(), 0, 0)
            .expect("Invalid time");
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };
    pub static ref NEXT_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-09", "%Y-%m-%d").unwrap();
        let datetime = date
            .and_hms_opt((Utc::now().hour() + 1) % 24, 0, 0)
            .expect("Invalid time");
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };
    pub static ref TODAY_THIS_HOUR: u32 = Utc::now().hour();
}
