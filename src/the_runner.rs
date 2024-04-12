use crate::db_join::*;
use crate::db_manipulate::*;
use crate::db_populate::*;
use crate::issue_tracker::*;
use flowsnet_platform_sdk::logger;
use lazy_static::lazy_static;

pub static ISSUE_LABEL: &str = "hacktoberfest";
pub static PR_LABEL: &str = "hacktoberfest-accepted";
pub static START_DATE: &str = "2023-10-04";
pub static END_DATE: &str = "2023-10-30";
use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use mysql_async::Pool;

lazy_static! {
    static ref THIS_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-01", "%Y-%m-%d").unwrap();
        let datetime = date.and_hms(Utc::now().hour(), 0, 0);
        datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
    };
    static ref NEXT_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-01", "%Y-%m-%d").unwrap();
        let datetime = date.and_hms((Utc::now().hour() + 1) % 24, 0, 0);
        datetime.format("%Y-%m-%dT%H:%M:%S").to_string()
    };
    static ref TODAY_THIS_HOUR: u32 = Utc::now().hour();
}

pub fn inner_query_1_hour(
    start_date: &str,
    start_hour: &str,
    end_hour: &str,
    issue_label: &str,
    pr_label: &str,
    is_issue: bool,
    is_assigned_issue: bool,
    is_start: bool,
) -> String {
    let date_range = format!("{}..{}", start_hour, end_hour);

    let query = if is_issue && is_start {
        format!("label:{issue_label} is:issue is:open no:assignee created:{date_range} -label:spam -label:invalid")
    } else if is_assigned_issue {
        format!("label:{issue_label} is:issue is:open is:assigned created:>={start_date} updated:{date_range} -label:spam -label:invalid")
    } else if is_issue && !is_start {
        format!("label:{issue_label} is:issue is:closed updated:{date_range} -label:spam -label:invalid")
    } else {
        format!("label:{pr_label} is:pr is:merged merged:{date_range} review:approved -label:spam -label:invalid")
    };

    query
}

pub fn inner_query_vec_by_date_range(
    start_date: &str,
    n_days: i64,
    start_hour: &str,
    end_hour: &str,
    issue_label: &str,
    pr_label: &str,
    is_issue: bool,
    is_assigned_issue: bool,
    is_start: bool,
) -> Vec<String> {
    let start_date =
        NaiveDate::parse_from_str(start_date, "%Y-%m-%d").expect("Failed to parse date");

    let date_range_vec = (1..n_days * 10) // 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19
        .step_by(n_days as usize) // 0 2 4 6 8 10 12 14 16 18
        .map(|i| (i + 1, i + n_days))
        .map(|(i, j)| {
            format!(
                "{}..{}",
                (start_date + Duration::days(i as i64))
                    .format("%Y-%m-%d")
                    .to_string(),
                (start_date + Duration::days(j as i64))
                    .format("%Y-%m-%d")
                    .to_string()
            )
        })
        .collect::<Vec<_>>();

    let mut out = Vec::new();

    for date_range in date_range_vec {
        let query = if is_issue && is_start {
            format!("label:{issue_label} is:issue is:open no:assignee created:{date_range} -label:spam -label:invalid")
        } else if is_assigned_issue {
            format!("label:{issue_label} is:issue is:open is:assigned created:>={start_date} updated:{date_range} -label:spam -label:invalid")
        } else if is_issue && !is_start {
            format!("label:{issue_label} is:issue is:closed updated:{date_range} -label:spam -label:invalid")
        } else {
            format!("label:{pr_label} is:pr is:merged merged:{date_range} review:approved -label:spam -label:invalid")
        };

        out.push(query);
    }

    out
}




pub async fn run_hourly(pool: &Pool) -> anyhow::Result<()> {
    logger::init();
    // let query_open ="label:hacktoberfest is:issue is:open no:assignee created:2023-10-01..2023-10-02 -label:spam -label:invalid";
    let query_open = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        true,
        false,
        true,
    );
    log::error!("query_open: {:?}", query_open);
    // let mock_user_obj: Vec<(String, String, String)> = search_mock_user(&query_open).await?;

    // let len = mock_user_obj.len();
    // log::error!("mock_user_obj: {:?}", len);
    // for (login, _, email) in mock_user_obj {
    //     let _ = add_mock_user(pool, &login, &email).await;
    // }

    // let open_issue_obj: Vec<IssueOpen> = search_issues_open(&query_open).await?;
    // let len = open_issue_obj.len();
    // log::error!("open_issue_obj: {:?}", len);
    // for issue in open_issue_obj {
    //     let _ = add_issues_open(pool, issue).await;
    // }

    // let query ="label:hacktoberfest is:issue is:open created:>=2023-10-01 updated:2023-10-03..2023-10-04 -label:spam -label:invalid";
    let query_comment = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        false,
        true,
        false,
    );
    // log::error!("query_comment: {:?}", query_comment);
    // let issues_comment_obj: Vec<IssueComments> =
    //     search_issues_w_update_comments(&query_comment).await?;
    // let len = issues_comment_obj.len();
    // log::error!("issues_comment_obj: {:?}", len);
    // for issue in issues_comment_obj {
    //     let _ = add_issues_comments(pool, issue).await;
    // }

    // let query_closed =
    //     "label:hacktoberfest is:issue is:closed updated:>=2023-10-01 -label:spam -label:invalid";
    let query_closed = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        true,
        false,
        false,
    );
    println!("query_closed: {:?}", query_closed);
    // let close_issue_obj = search_issues_closed(&query_closed).await?;
    // let len = close_issue_obj.len();
    // log::error!("close_issue_obj: {:?}", len);
    // for issue in close_issue_obj {
    //     let _ = add_issues_closed(pool, issue).await;
    // }

    // let query_pr_overall ="label:hacktoberfest-accepted is:pr is:merged updated:2023-10-01..2023-10-02 review:approved -label:spam -label:invalid";
    // let query_pull_request = inner_query_1_hour(
    //     &START_DATE,
    //     &THIS_HOUR,
    //     &NEXT_HOUR,
    //     ISSUE_LABEL,
    //     PR_LABEL,
    //     false,
    //     false,
    //     false,
    // );
    // log::error!("query_pull_request: {:?}", query_pull_request);
    // let pull_request_obj: Vec<OuterPull> = search_pull_requests(&query_pull_request).await?;
    // let len = pull_request_obj.len();
    // log::error!("pull_request_obj: {:?}", len);
    // for pull in pull_request_obj {
    //     let _ = add_pull_request(&pool, pull).await;
    // }

    let _ = open_master(&pool).await?;

    let _ = closed_master(&pool).await?;

    let _ = open_master_project(&pool).await?;

    Ok(())
}
