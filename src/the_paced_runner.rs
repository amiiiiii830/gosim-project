use crate::{db_join::*, db_manipulate::*, db_populate::*, issue_tracker::*, vector_search::*};
use crate::{ISSUE_LABEL, NEXT_HOUR, PR_LABEL, START_DATE, THIS_HOUR};

use anyhow::Ok;
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

pub async fn run_hourly(pool: &Pool) -> anyhow::Result<()> {
    let _ = popuate_dbs_save_issues_open(pool).await?;

    let _ = open_master(pool).await?;

    let _ = popuate_dbs_save_issues_assigned(pool).await?;

    let _ = assigned_master(pool).await?;

    let _ = popuate_dbs_save_issues_closed(pool).await?;

    let _ = closed_master(pool).await?;

    let _ = popuate_dbs_fill_projects(pool).await?;

    let _ = master_project(&pool).await?;

    let _ = popuate_dbs_save_pull_requests(pool).await?;

    let _ = project_master_back_sync(&pool).await?;

    let _ = populate_vector_db(pool).await?;

    let _ = popuate_dbs_save_issues_comment(pool).await?;

    let _ = sum_budget_to_project(&pool).await?;

    let _ = remove_pull_by_issued_linked_pr(&pool).await?;
    let _ = delete_issues_open_assigned_closed(&pool).await?;

    // let _ = note_issues(pool).await?;

    Ok(())
}
pub async fn popuate_dbs_save_issues_open(pool: &Pool) -> anyhow::Result<()> {
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
        let _ = add_issues_open(pool, &issue).await;

        let _ = summarize_issue_add_in_db(pool, &issue).await;
    }
    Ok(())
}
pub async fn popuate_dbs_save_issues_comment(pool: &Pool) -> anyhow::Result<()> {
    let query_comment =
        "label:hacktoberfest-accepted is:issue updated:>2024-01-01 -label:spam -label:invalid";
    log::info!("query_open: {:?}", query_comment);

    let issue_comment_obj: Vec<IssueComment> = search_issues_comment(&query_comment).await?;
    let len = issue_comment_obj.len();
    log::info!("Issues comment recorded: {:?}", len);
    for issue in issue_comment_obj {
        let _ = add_issues_comment(pool, issue).await;
    }
    Ok(())
}
pub async fn popuate_dbs_save_issues_assigned(pool: &Pool) -> anyhow::Result<()> {
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
    Ok(())
}
pub async fn popuate_dbs_save_issues_closed(pool: &Pool) -> anyhow::Result<()> {
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

    Ok(())
}

pub async fn popuate_dbs_save_pull_requests(pool: &Pool) -> anyhow::Result<()> {
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

pub async fn popuate_dbs_fill_projects(pool: &Pool) -> anyhow::Result<()> {
    let query_repos: String = get_projects_as_repo_list(pool, 1).await?;

    let repo_data_vec: Vec<RepoData> = search_repos_in_batch(&query_repos).await?;

    let repo_ids_vec = repo_data_vec
        .clone()
        .into_iter()
        .map(|r| r.project_id)
        .collect::<Vec<String>>();
    let len = repo_data_vec.len();
    let repo_ids = repo_ids_vec.join("\n");

    log::info!("{len} repo_ids: {:?}", repo_ids);

    for repo_data in repo_data_vec {
        let _ = fill_project_w_repo_data(&pool, repo_data.clone()).await?;
        let _ = summarize_project_add_in_db(&pool, repo_data).await?;
    }
    Ok(())
}

pub async fn populate_vector_db(pool: &Pool) -> anyhow::Result<()> {
    for item in get_issues_repos_from_db().await.expect("msg") {
        log::info!("uploading to vector_db: {:?}", item.0);
        let _ = upload_to_collection(&item.0, item.1.clone()).await;
        let _ = mark_id_indexed(&pool, &item.0).await;
    }
    let _ = check_vector_db("gosim_search").await;

    Ok(())
}

pub async fn note_issues(pool: &Pool) -> anyhow::Result<()> {
    let _ = note_budget_allocated(pool).await?;
    let _ = note_issue_declined(pool).await?;
    let _ = note_distribute_fund(pool).await?;
    let _ = note_one_months_no_pr(pool).await?;
    Ok(())
}

pub async fn note_budget_allocated(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_with_budget(pool).await?;
    log::info!(
        "Issue ids with budget allocated, count: {:?}",
        issue_ids.len()
    );
    for issue_id in issue_ids {
        let comment = format!("{}/n Congratulations! GOSIM grant approved. Your proposal is approved to get $100 fund to fix the issue.", issue_id);

        // let _ = mock_comment_on_issue(1, &comment).await?;
    }
    Ok(())
}

pub async fn note_issue_declined(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_declined(pool).await?;
    log::info!(
        "Issue ids with budget declined, count: {:?}",
        issue_ids.len()
    );
    for issue_id in issue_ids {
        let comment = format!("{}/n  I’m sorry your proposal wasn't approved", issue_id);

        // let _ = mock_comment_on_issue(2, &comment).await?;
    }
    Ok(())
}

pub async fn note_distribute_fund(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids: Vec<(Option<String>, String, i32)> = get_issue_ids_distribute_fund(pool).await?;
    log::info!("Issue_ids to split fund, count: {:?}", issue_ids.len());
    for (issue_assignee, _issue_id, issue_budget) in issue_ids {
        let comment = format!("@{:?}, Well done!  According to the PR commit history. @{:?} should receive ${}. Please fill in this form to claim your fund. ", issue_assignee, issue_assignee, issue_budget);

        // let _ = mock_comment_on_issue(3, &comment).await?;
    }
    Ok(())
}

pub async fn note_one_months_no_pr(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_one_month_no_activity(pool).await?;
    log::info!("Issue_ids no activity, count: {:?}", issue_ids.len());

    for issue_id in issue_ids {
        let comment = format!("{}\n @{} please link your PR to the issue it fixed in three days. Or this issue will be deemed not completed, then we can’t provide the fund.", issue_id, "issue_assignee" );

        // let _ = mock_comment_on_issue(4, &comment).await?;
    }
    Ok(())
}
