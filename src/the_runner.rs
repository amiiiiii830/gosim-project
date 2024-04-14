use crate::{db_join::*, db_manipulate::*, db_populate::*, issue_tracker::*};
use crate::{ISSUE_LABEL, NEXT_HOUR, PR_LABEL, START_DATE, THIS_HOUR};

use mysql_async::Pool;

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
    let _ = popuate_dbs(pool).await?;
    let _ = join_ops(pool).await?;
    Ok(())
}
pub async fn popuate_dbs(pool: &Pool) -> anyhow::Result<()> {
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
    log::info!("Assigned issues recorded: {:?}", len);
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
    log::info!("query_closed: {:?}", query_closed);
    let close_issue_obj = search_issues_closed(&query_closed).await?;
    let len = close_issue_obj.len();
    log::info!("Closed issues recorded: {:?}", len);
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
    log::info!("query_pull_request: {:?}", query_pull_request);
    let pull_request_obj: Vec<OuterPull> = search_pull_requests(&query_pull_request).await?;
    let len = pull_request_obj.len();
    log::info!("Pull requests recorded: {:?}", len);
    for pull in pull_request_obj {
        let _ = add_pull_request(&pool, pull).await;
    }

    Ok(())
}

pub async fn join_ops(pool: &Pool) -> anyhow::Result<()> {
    let _ = open_master(&pool).await?;
    let _ = assigned_master(&pool).await?;

    let _ = closed_master(&pool).await?;

    // let _ = pull_master(&pool).await?;
    let _ = master_project(&pool).await?;

    let query_repos: String = get_projects_as_repo_list(pool, 1).await?;

    let repo_data_vec: Vec<RepoData> = search_repos_in_batch(&query_repos).await?;

    for repo_data in repo_data_vec {
        let _ = fill_project_w_repo_data(&pool, repo_data).await?;
    }
    Ok(())
}
