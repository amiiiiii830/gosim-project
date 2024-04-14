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

pub async fn list_issues(
    pool: &Pool,
    page: usize,
    page_size: usize,
) -> Result<Vec<IssueSubset>> {
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

pub async fn list_projects(
    pool: &Pool,
    page: usize,
    page_size: usize,
) -> Result<Vec<Project>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;
    let projects: Vec<Project> = conn
        .query_map(
            format!(
                "SELECT project_id, project_logo, repo_stars, project_description, issues_list,   total_budget_allocated, total_budget_used FROM projects ORDER BY project_id LIMIT {} OFFSET {}",
                page_size, offset
            ),
            |(project_id, project_logo, repo_stars, project_description, issues_list,  total_budget_allocated, total_budget_used): (String, Option<String>, i32, Option<String>, Option<String>,Option<i32>, Option<i32>)| {
                Project {
                    project_id,
                    project_logo,
                    repo_stars,
                    project_description,
                    issues_list: issues_list.map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),
                    total_budget_allocated,
                    total_budget_used
                }
            },
        )
        .await?;

    Ok(projects)
}

pub async fn select_issue(
    pool: &mysql_async::Pool,
    issue_id: &str,
    issue_budget: i64,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues 
                  SET issue_budget = :issue_budget, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
                "issue_budget" => issue_budget,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn approve_issue(
    pool: &mysql_async::Pool,
    issue_id: &str,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues 
                  SET issue_budget_approved = True, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn approve_issue_budget_in_db(
    pool: &mysql_async::Pool,
    issue_id: &str,
    issue_budget: i64,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget = :issue_budget, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
                "issue_budget" => issue_budget,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn conclude_issue_in_db(pool: &mysql_async::Pool, issue_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget_approved = True
                  WHERE issue_id = :issue_id";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}
