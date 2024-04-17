use crate::db_populate::*;
use mysql_async::prelude::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueSubset {
    pub issue_id: String,
    pub project_id: String,
    pub issue_title: String,
    pub issue_budget: Option<i32>,
    pub issue_status: Option<String>,
    pub review_status: ReviewStatus,
    pub issue_budget_approved: bool,
}

pub async fn batch_decline_issues_in_db(pool: &Pool, issue_ids: Vec<String>) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    for issue_id in issue_ids {
        if let Err(e) = conn
            .exec_drop(
                r"UPDATE issues_master SET review_status = 'decline' WHERE issue_id = :issue_id",
                params! {
                    "issue_id" => &issue_id
                },
            )
            .await
        {
            log::error!("Error batch decline issues: {:?}", e);
        }
    }

    Ok(())
}

pub async fn list_issues_by_status(
    pool: &Pool,
    review_status: &str,
    page: usize,
    page_size: usize,
) -> Result<Vec<IssueOut>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;
    let issues: Vec<IssueOut> = conn
        .query_map(
            format!(
                "SELECT issue_id, project_id, issue_title, issue_description, issue_budget, issue_assignees, issue_linked_pr, issue_status, review_status, issue_budget_approved FROM issues_master where review_status = '{}' ORDER BY issue_id LIMIT {} OFFSET {}",
                review_status, page_size, offset
            ),
            |(issue_id, project_id, issue_title, issue_description, issue_budget, issue_assignees_value, issue_linked_pr, issue_status, review_status, issue_budget_approved): (String, String, String, String, Option<i32>, Option<String>, Option<String>, Option<String>, String, Option<bool>)| {
                let issue_assignees = match &issue_assignees_value {
                    Some(value) => {
                        let vec: Vec<String> = serde_json::from_str(value).unwrap_or_default();
                        Some(vec)
                    }
                    None => None,
                };
                IssueOut {
                    issue_id,
                    project_id,
                    issue_title,
                    issue_description,
                    issue_budget,
                    issue_assignees,
                    issue_linked_pr,
                    issue_status: issue_status,
                    review_status,
                    issue_budget_approved: issue_budget_approved.unwrap_or_default(),
                }
            },
        )
        .await?;

    Ok(issues)
}

pub async fn list_issues(pool: &Pool, page: usize, page_size: usize) -> Result<Vec<IssueSubset>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;
    let issues: Vec<IssueSubset> = conn
        .query_map(
            format!(
                "SELECT issue_id, project_id, issue_title, issue_budget, issue_status, review_status, issue_budget_approved FROM issues_master ORDER BY issue_id LIMIT {} OFFSET {}",
                page_size, offset
            ),
            |(issue_id, project_id, issue_title, issue_budget, issue_status, review_status, issue_budget_approved): (String, String, String, Option<i32>, Option<String>, Option<String>, Option<bool>)| {
                IssueSubset {
                    issue_id,
                    project_id,
                    issue_title,
                    issue_budget,
                    issue_status,
                    review_status: match review_status.unwrap_or_default().as_str() {
                        "queue" => ReviewStatus::Queue,
                        "approve" => ReviewStatus::Approve,
                        "decline" => ReviewStatus::Decline,
                        _ => ReviewStatus::Queue,
                    },
                    issue_budget_approved: issue_budget_approved.unwrap_or_default(),
                }
            },
        )
        .await?;

    Ok(issues)
}
// "SELECT project_id FROM projects WHERE project_logo is NULL ORDER BY project_id LIMIT :limit OFFSET :offset",

pub async fn get_projects_as_repo_list(pool: &Pool, page: u32) -> Result<String> {
    let page_size = 30u32;
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;
    let project_ids: Vec<String> = conn
        .exec_map(
            "SELECT project_id FROM projects ORDER BY project_id LIMIT :limit OFFSET :offset",
            params! {
                "limit" => page_size,
                "offset" => offset,
            },
            |project_id: String| project_id,
        )
        .await?;

    let res = project_ids
        .into_iter()
        .map(|x| {
            let owner_repo = x.replace("https://github.com/", "repo:");

            owner_repo
        })
        .collect::<Vec<String>>();

    Ok(res.join(" "))
}

pub async fn list_projects(pool: &Pool, page: usize, page_size: usize) -> Result<Vec<Project>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;
    let projects: Vec<Project> = conn
        .query_map(
            format!(
                "SELECT project_id, project_logo, repo_stars, project_description, issues_list,   total_budget_allocated, total_budget_used FROM projects ORDER BY project_id LIMIT {} OFFSET {}",
                page_size, offset
            ),
            |(project_id, project_logo, repo_stars, project_description, issues_list,  total_budget_allocated ): (String, Option<String>, i32, Option<String>, Option<String>,Option<i32>)| {
                Project {
                    project_id,
                    project_logo,
                    repo_stars,
                    project_description,
                    issues_list: issues_list.map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),
                    total_budget_allocated
                }
            },
        )
        .await?;

    Ok(projects)
}

use mysql_async::prelude::FromRow;
use mysql_async::Row;
use mysql_async::Value;

impl FromRow for IssueOut {
    fn from_row_opt(row: Row) -> std::result::Result<Self, mysql_async::FromRowError> {
        let (
            issue_id,
            project_id,
            issue_title,
            issue_description,
            issue_budget,
            issue_assignees_value,
            issue_linked_pr,
            issue_status,
            review_status,
            issue_budget_approved,
        ) = mysql_async::from_row_opt(row)?;

        // Convert issue_assignees_value into Vec<String>
        let issue_assignees = match issue_assignees_value {
            Value::Bytes(bytes) => {
                let s = String::from_utf8_lossy(&bytes);
                let vec: Vec<String> = serde_json::from_str(&s).unwrap_or_default();
                Some(vec)
            }
            _ => None,
        };

        Ok(IssueOut {
            issue_id,
            project_id,
            issue_title,
            issue_description,
            issue_budget,
            issue_assignees,
            issue_linked_pr,
            issue_status,
            review_status,
            issue_budget_approved,
        })
    }
}
pub async fn get_issue_by_id(pool: &Pool, issue_id: &str) -> anyhow::Result<IssueOut> {
    let mut conn = pool.get_conn().await?;

    let query = r"SELECT issue_id, project_id, issue_title, issue_description, issue_budget, issue_assignees, issue_linked_pr, issue_status, review_status, issue_budget_approved FROM issues_master WHERE issue_id = :issue_id";

    match conn
        .exec_first(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        Ok(Some(issue)) => Ok(issue),
        Ok(None) => Err(anyhow::anyhow!("No issue found with the provided issue_id")),
        Err(e) => {
            log::error!("Error getting issue by issue_id: {:?}", e);
            Err(anyhow::anyhow!("Error getting issue by issue_id"))
        }
    }
}
pub async fn get_issue_ids_with_budget(pool: &Pool) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;
    // let one_hour_ago = (Utc::now() - Duration::try_hours(1).unwrap())
    //     .naive_utc()
    //     .format("%Y-%m-%d %H:%M:%S")
    //     .to_string();
    let one_hour_ago = "2023-10-06 13:04:00".to_string();

    let selected_rows: Vec<String> = conn
        .exec_map(
            "SELECT issue_id FROM issues_master WHERE issue_budget > 0 AND review_status='approve' AND date_issue_assigned > :one_hour_ago",
                        params! {
                "one_hour_ago" => &one_hour_ago
            },
            |issue_id| issue_id,
        )
        .await?;
    Ok(selected_rows)
}

pub async fn get_issue_ids_declined(pool: &Pool) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;
    let selected_rows: Vec<String> = conn
        .query_map(
            "select issue_id from issues_master where review_status='decline' limit 5;",
            |issue_id| issue_id,
        )
        .await?;
    Ok(selected_rows)
}

pub async fn get_issue_ids_distribute_fund(
    pool: &Pool,
) -> Result<Vec<(Option<String>, String, i32)>> {
    let mut conn = pool.get_conn().await?;
    let selected_rows: Vec<(Option<String>, String, i32)> = conn
        .query_map(
            "SELECT issue_assignees, issue_id, issue_budget FROM issues_master WHERE issue_budget_approved=1 LIMIT 5",
            |(issue_assignees, issue_id, issue_budget): (Option<String>, Option<String>, Option<i32>)| {
                let issue_assignee = issue_assignees.and_then(|s| s.split(",").next().map(String::from));
                (issue_assignee, issue_id.unwrap_or_default(), issue_budget.unwrap_or(0))
            },
        )
        .await?;
    Ok(selected_rows)
}
pub async fn get_issue_ids_one_month_no_activity(pool: &Pool) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;
    // let _one_month_ago =
    //     (Utc::now() - Duration::try_days(30).unwrap().to_std().unwrap()).naive_utc();
    // let _formatted_one_month_ago = _one_month_ago.format("%Y-%m-%d %H:%M:%S").to_string();
    let formatted_one_month_ago = "2023-10-06 13:04:00".to_string();

    let selected_rows: Vec<String> = conn.exec_map(
        "SELECT issue_id FROM issues_master WHERE date_issue_assigned < :formatted_one_month_ago AND issue_linked_pr IS NULL",
        params! {
            "one_month_ago" => formatted_one_month_ago,
        },
        |issue_id| issue_id,
    ).await?;
    Ok(selected_rows)
}

pub async fn assign_issue_budget_in_db(
    pool: &mysql_async::Pool,
    issue_id: &str,
    issue_budget: i64,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget = :issue_budget, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
                "issue_budget" => issue_budget,
            },
        )
        .await
    {
        log::error!("Error assign issue_budget: {:?}", e);
    };

    Ok(())
}

pub async fn decline_issue_in_db(pool: &mysql_async::Pool, issue_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget = null, 
                      review_status = 'decline'
                  WHERE issue_id = :issue_id";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        log::error!("Error decline issue: {:?}", e);
    };

    Ok(())
}

pub async fn decline_issues_batch_in_db(
    pool: &mysql_async::Pool,
    issue_ids: Vec<&str>,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget = null, 
                      review_status = 'decline'
                  WHERE issue_id = :issue_id";

    for issue_id in issue_ids {
        if let Err(e) = conn
            .exec_drop(
                query,
                params! {
                    "issue_id" => issue_id,
                },
            )
            .await
        {
            log::error!("Error batch decline issues: {:?}", e);
        };
    }

    Ok(())
}

pub async fn conclude_issue_in_db(pool: &mysql_async::Pool, issue_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget_approved = True
                  WHERE issue_id = :issue_id";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        log::error!("Error concluding issue: {:?}", e);
    };

    Ok(())
}

pub async fn conclude_issues_batch_in_db(
    pool: &mysql_async::Pool,
    issue_ids: Vec<&str>,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget_approved = True
                  WHERE issue_id = :issue_id";

    for issue_id in issue_ids {
        if let Err(e) = conn
            .exec_drop(
                query,
                params! {
                    "issue_id" => issue_id,
                },
            )
            .await
        {
            log::error!("Error concluding issues batch: {:?}", e);
        };
    }

    Ok(())
}
