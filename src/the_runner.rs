use crate::{
    db_join::*,
    db_populate::*,
    db_populate::{add_issues_assigned, add_issues_closed, add_pull_request},
    issue_tracker::*,
};
use flowsnet_platform_sdk::logger;
use lazy_static::lazy_static;

pub static ISSUE_LABEL: &str = "hacktoberfest";
pub static PR_LABEL: &str = "hacktoberfest-accepted";
pub static START_DATE: &str = "2023-10-03";
pub static END_DATE: &str = "2023-10-30";
use chrono::{Duration, NaiveDate, Timelike, Utc};
use mysql_async::Pool;

lazy_static! {
    static ref THIS_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-03", "%Y-%m-%d").unwrap();
        let datetime = date
            .and_hms_opt(Utc::now().hour(), 0, 0)
            .expect("Invalid time");
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    };
    static ref NEXT_HOUR: String = {
        let date = NaiveDate::parse_from_str("2023-10-03", "%Y-%m-%d").unwrap();
        let datetime = date
            .and_hms_opt((Utc::now().hour() + 1) % 24, 0, 0)
            .expect("Invalid time");
        datetime.format("%Y-%m-%dT%H:%M:%SZ").to_string()
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
        format!("label:{issue_label} is:issue is:closed created:>{start_date} closed:{date_range} -label:spam -label:invalid")
    } else if is_assigned_issue {
        format!("label:{issue_label} is:issue is:closed created:>{start_date} closed:{date_range} -label:spam -label:invalid")
    } else if is_issue && !is_start {
        format!("label:{issue_label} is:issue is:closed created:>{start_date} closed:{date_range} -label:spam -label:invalid")
    } else {
        format!("label:{pr_label} is:pr is:merged merged:{date_range} review:approved -label:spam -label:invalid")
    };

    // let query = if is_issue && is_start {
    //     format!("label:{issue_label} is:issue is:open no:assignee created:{date_range} -label:spam -label:invalid")
    // } else if is_assigned_issue {
    //     format!("label:{issue_label} is:issue is:open is:assigned created:>={start_date} updated:{date_range} -label:spam -label:invalid")
    // } else if is_issue && !is_start {
    //     format!("label:{issue_label} is:issue is:closed updated:{date_range} -label:spam -label:invalid")
    // } else {
    //     format!("label:{pr_label} is:pr is:merged merged:{date_range} review:approved -label:spam -label:invalid")
    // };

    query
}

/* pub fn inner_query_vec_by_date_range(
    start_date: &str,
    n_days: i64,
    _start_hour: &str,
    _end_hour: &str,
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
                (start_date + Duration::try_days(i as i64).expect("Invalid number of days"))
                    .format("%Y-%m-%d")
                    .to_string(),
                (start_date + Duration::try_days(j as i64).expect("Invalid number of days"))
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
} */

pub async fn run_hourly(pool: &Pool) -> anyhow::Result<()> {
    logger::init();
    let query_open = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        true,  // is_issue
        false, // is_assigned_issue
        true,  // is_start
    );
    log::info!("query_open: {:?}", query_open);

    let open_issue_obj: Vec<IssueOpen> = search_issues_open(&query_open).await?;
    let len = open_issue_obj.len();
    log::info!("Open Issues recorded: {:?}", len);
    for issue in open_issue_obj {
        let _ = add_issues_open(pool, issue).await;
    }

    let _query_assigned = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        false,
        true,
        false,
    );

    log::info!("query_assigned: {:?}", _query_assigned);
    let issues_assigned_obj: Vec<IssueAssigned> = search_issues_assigned(&_query_assigned).await?;
    let len = issues_assigned_obj.len();
    log::error!("Assigned issues recorded: {:?}", len);
    for issue in issues_assigned_obj {
        let _ = add_issues_assigned(pool, issue).await;
    }

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
    let close_issue_obj = search_issues_closed(&query_closed).await?;
    let len = close_issue_obj.len();
    log::error!("Closed issues recorded: {:?}", len);
    for issue in close_issue_obj {
        let _ = add_issues_closed(pool, issue).await;
    }

    let query_pull_request = inner_query_1_hour(
        &START_DATE,
        &THIS_HOUR,
        &NEXT_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        false,
        false,
        false,
    );
    log::error!("query_pull_request: {:?}", query_pull_request);
    let pull_request_obj: Vec<OuterPull> = search_pull_requests(&query_pull_request).await?;
    let len = pull_request_obj.len();
    log::error!("Pull requests recorded: {:?}", len);
    for pull in pull_request_obj {
        let _ = add_pull_request(&pool, pull).await;
    }

    let _ = open_master(&pool).await?;
    let _ = assigned_master(&pool).await?;

    let _ = closed_master(&pool).await?;

    let _ = pull_master(&pool).await?;

    Ok(())
}
