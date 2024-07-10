//in this iteration of the module, key functions that searches GitHub, summarizes content, populate databases, etc.
//are scheduled to run in a more granular fashsion, so that one task won't jam the other
//so that each function can handle an anticipated amount of work quickly, keeping the total run time far below one hour 
use crate::{
    db_join::*, db_manipulate::*, db_populate::*, issue_bot::comment_on_issue,
    issue_paced_tracker::*, vector_search::*,
};
use crate::{ISSUE_LABEL, PREV_HOUR, PR_LABEL, START_DATE, THIS_HOUR};

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
    let date_range =
        std::env::var("DATE_RANGE").unwrap_or_else(|_| format!("{}..{}", start_hour, end_hour));
    let signature_switch =
        std::env::var("SIGNATURE_SWITCH").unwrap_or_else(|_| "signature_switch".to_string());

    let query = if is_issue && is_start {
        format!("label:{issue_label} is:issue is:open no:assignee created:{date_range} -label:spam -label:invalid")
    } else if is_assigned_issue {
        format!("label:{issue_label} is:issue is:open created:>{start_date} updated:{date_range} -label:spam -label:invalid")
    } else if is_issue && !is_start {
        format!("label:{issue_label} is:issue is:closed created:>{start_date} closed:{date_range} -label:spam -label:invalid")
    } else {
        format!("label:{pr_label} is:pr is:merged merged:{date_range} review:approved -label:spam -label:invalid")
    };

    query
}
pub async fn run_hourly(pool: &Pool) -> anyhow::Result<()> {

    //search issues newly opened in the past hour, save them to issues_open table in db, this table holds data temporarily 
    let _ = popuate_dbs_save_issues_open(pool).await?;

    //replicate issue entries in issues_open table to the issues_master table, which serves as long term record 
    //the task is done directly on db, in a batch transaction fashion, intended to avoid db transactions slowing down other tasks
    let _ = open_master(pool).await?;

    //replicate issues data in issues_open table to projects table
    let _ = open_project(pool).await?;

    //search for detailed info of projects(repos) in batch, save data to the projects table to make it richer
    let _ = popuate_dbs_fill_projects(pool).await?;

    //now that the projects table has more detailed info, we replicate several fields to the issues_master table
    let _ = project_master_back_sync(&pool).await?;

    //search for issues closed in past hour, save them in issues_closed table, which holds data temporariliy
    let _ = popuate_dbs_save_issues_closed(pool).await?;

    //replicate issue entries in issues_closed table to issues_master table
    let _ = closed_master(pool).await?;

    //now that we have detailed info like project descriptions, issue body text, etc. we upload them to vector db for future querying
    let _ = populate_vector_db(pool).await?;

    //issues may have updated budget info in the past hour, we run through the issues_master table and update the data in projects table
    let _ = sum_budget_to_project(&pool).await?;

    //search for issues updated in the past hour, save them to issues_updated table
    let _ = popuate_dbs_add_issues_updated(pool).await?;

    //search for issues with new comments, changed assignee status in the past hour, save them to issues_assign_comment table
    let _ = popuate_dbs_save_issues_assign_comment(pool).await?;

    //issues may have assignees in the past hour, we use data in issues_assign_comment table to update issues_master table
    let _ = add_possible_assignees_to_master(pool).await?;

    //search for pull_requests in the past hour, save them in pull_requests table
    let _ = popuate_dbs_save_pull_requests(pool).await?;

    //in pull_requests table, some of them now has linked issue, we update issues_master table, remove these entries from pull_request table
    let _ = remove_pull_by_issued_linked_pr(&pool).await?;

    //approaching the end of the hourly project run, we empty temporary tables issues_open, issues_updated, and issues_closed
    let _ = delete_issues_open_update_closed(&pool).await?;

    //run through the issues_master table, identify those needing follow-up messages, post messages to those issues
    let _ = note_issues(pool).await?;

    Ok(())
}

//search for issues opened in past hour
pub async fn popuate_dbs_save_issues_open(pool: &Pool) -> anyhow::Result<()> {
    //we build a query that searches for issues opened in the past hour, with additonal conditions, i.e. no assignees
    let query_open = inner_query_1_hour(
        &START_DATE,
        &PREV_HOUR,
        &THIS_HOUR,
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

        //issues may have long descriptions, we summarize them and save them to issues_repos_summarized table
        let _ = summarize_issue_add_in_db(pool, &issue).await;
    }
    Ok(())
}

/* pub async fn force_issue_to_summary_update_db(pool: &Pool) -> anyhow::Result<()> {
    for page in 2..10 {
        let open_issue_obj: Vec<IssueOpen> = get_issues_open_from_master(pool, page).await?;
        let len = open_issue_obj.len();
        log::info!(
            "Simulate Open Issues retrieved from issues_master: {:?}",
            len
        );
        for issue in open_issue_obj {
            let _ = summarize_issue_add_in_db(pool, &issue).await;
        }
    }

    Ok(())
} */

/* pub async fn force_issue_to_summary_update_db(pool: &Pool) -> anyhow::Result<()> {
    for page in 2..10 {
        let open_issue_obj: Vec<IssueOpen> = get_issues_open_from_master(pool, page).await?;
        let len = open_issue_obj.len();
        log::info!(
            "Simulate Open Issues retrieved from issues_master: {:?}",
            len
        );
        for issue in open_issue_obj {
            let _ = summarize_issue_add_in_db(pool, &issue).await;
        }
    }

    Ok(())
}
 */


//search for issues with new comments, or changed assignee status in the past hour
pub async fn popuate_dbs_save_issues_assign_comment(pool: &Pool) -> anyhow::Result<()> {
    let node_ids_updated = get_updated_approved_issues_node_ids(pool).await?;

    for node_ids in node_ids_updated.chunks(30) {
        let node_ids = node_ids.to_vec();
        log::info!("node ids updated: {:?}", node_ids.clone());

        let issue_comment_obj: Vec<IssueAssignComment> =
            search_issues_assign_comment(node_ids).await?;
        let len = issue_comment_obj.len();
        log::info!("Issues assign, comment recorded: {:?}", len);
        for issue in issue_comment_obj {
            let _ = add_issues_assign_comment(pool, issue).await;
        }
    }

    Ok(())
}

//search for issues updated in the past hour, save them to issues_udpated table
pub async fn popuate_dbs_add_issues_updated(pool: &Pool) -> anyhow::Result<()> {
    let _query_assigned = inner_query_1_hour(
        &START_DATE,
        &PREV_HOUR,
        &THIS_HOUR,
        ISSUE_LABEL,
        PR_LABEL,
        false,
        true,
        false,
    );

    log::info!("query_assigned: {:?}", _query_assigned);
    let issues_assigned_obj: Vec<IssueUpdated> = search_issues_updated(&_query_assigned).await?;
    let len = issues_assigned_obj.len();
    log::info!("Updated issues recorded: {:?}", len);
    for issue in issues_assigned_obj {
        let _ = add_issues_updated(pool, issue).await;
    }
    Ok(())
}

//search for issues closed in the past hour, save them to issues_closed table
pub async fn popuate_dbs_save_issues_closed(pool: &Pool) -> anyhow::Result<()> {
    let query_closed = inner_query_1_hour(
        &START_DATE,
        &PREV_HOUR,
        &THIS_HOUR,
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


//search for pull_requests in the past hour, save them to pull_requests table 
pub async fn popuate_dbs_save_pull_requests(pool: &Pool) -> anyhow::Result<()> {
    let query_pull_request = inner_query_1_hour(
        &START_DATE,
        &PREV_HOUR,
        &THIS_HOUR,
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

//search for detailed info about projects(repos) with their identifiers, save them to the projects table
pub async fn popuate_dbs_fill_projects(pool: &Pool) -> anyhow::Result<()> {
    let query_repos: String = get_projects_as_repo_list(pool, 1).await?;
    let repo_data_vec: Vec<RepoData> = search_repos_in_batch(&query_repos).await?;

    for repo_data in repo_data_vec {
        let _ = fill_project_w_repo_data(&pool, repo_data.clone()).await?;
        let _ = summarize_project_add_in_db(&pool, repo_data).await?;
    }
    Ok(())
}


//issues and projects have been summarized previously and saved in the issues_repos_summarized table
//now we use these clean texts to populate the vector db 
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


//run through the issues_master table, locate issues that have seen budget allocation the past hour
//post comments on these issues to notify project participants
pub async fn note_budget_allocated(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_with_budget(pool).await?;
    log::info!(
        "Issue ids with budget allocated, count: {:?}",
        issue_ids.len()
    );
    for (issue_id, issue_budget) in issue_ids {
        let comment = format!("Congratulations! GOSIM grant approved. Your proposal is approved to get ${} fund to fix the issue.", issue_budget);

        let _ = comment_on_issue(&issue_id, &comment).await?;
    }
    Ok(())
}

//run through the issues_master table, locate issues that have been declined in the past hour
//post comments on these issues to notify project participants
pub async fn note_issue_declined(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_declined(pool).await?;
    log::info!(
        "Issue ids with budget declined, count: {:?}",
        issue_ids.len()
    );
    for issue_id in issue_ids {
        let comment = format!("I’m sorry your proposal wasn't approved");

        let _ = comment_on_issue(&issue_id, &comment).await?;
    }
    Ok(())
}

//run through the issues_master table, locate issues that have been flagged issue_budget_approved in the past hour
//post comments on these issues to notify project participants
pub async fn note_distribute_fund(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids: Vec<(Option<String>, String, i32)> = get_issue_ids_distribute_fund(pool).await?;
    log::info!("Issue_ids to split fund, count: {:?}", issue_ids.len());
    for (issue_assignee, _issue_id, issue_budget) in issue_ids {
        let comment = format!("@{:?}, Well done!  According to the PR commit history. @{:?} should receive ${}. Please fill in this form to claim your fund. ", issue_assignee, issue_assignee, issue_budget);

        let _ = comment_on_issue(&_issue_id, &comment).await?;
    }
    Ok(())
}

//run through the issues_master table, locate issues that have been allocated budget but saw no activity in the past month
//post comments on these issues to notify project participants
pub async fn note_one_months_no_pr(pool: &Pool) -> anyhow::Result<()> {
    let issue_ids = get_issue_ids_one_month_no_activity(pool).await?;
    log::info!("Issue_ids no activity, count: {:?}", issue_ids.len());

    for issue_id in issue_ids {
        let comment = format!("please link your PR to the issue it fixed in three days. Or this issue will be deemed not completed, then we can’t provide the fund.");

        let _ = comment_on_issue(&issue_id, &comment).await?;
    }
    Ok(())
}
