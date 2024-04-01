use crate::issue_tracker::*;
use dotenv::dotenv;
use mysql_async::prelude::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueMaster {
    pub issue_id: String,
    pub project_id: String,
    pub issue_title: String,
    pub issue_description: String,
    pub issue_budget: Option<i32>,
    pub issue_assignees: Option<Vec<String>>, // or a more specific type if you know the structure of the JSON
    pub issue_linked_pr: Option<String>,
    pub issue_status: String,
    pub review_status: ReviewStatus,
    pub issue_budget_approved: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub enum ReviewStatus {
    #[default]
    Queue,
    Approve,
    Decline,
}

pub async fn get_pool() -> Pool {
    dotenv().ok();
    let url = std::env::var("DATABASE_URL").expect("not url db url found");

    let opts = Opts::from_url(&url).unwrap();
    let builder = OptsBuilder::from_opts(opts);
    // The connection pool will have a min of 5 and max of 10 connections.
    let constraints = PoolConstraints::new(5, 10).unwrap();
    let pool_opts = PoolOpts::default().with_constraints(constraints);

    Pool::new(builder.pool_opts(pool_opts))
}

pub async fn project_exists(pool: &mysql_async::Pool, project_id: &str) -> Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<(i32,)> = conn
        .query_first(format!(
            "SELECT 1 FROM projects WHERE project_id = '{}'",
            project_id
        ))
        .await?;
    Ok(result.is_some())
}

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

    conn.exec_drop(
        query,
        params! {
            "issue_id" => issue_id,
            "issue_budget" => issue_budget,
        },
    )
    .await?;

    Ok(())
}

pub async fn approve_issue(pool: &mysql_async::Pool, issue_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues 
                  SET issue_budget_approved = True, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    let _result = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await;

    Ok(())
}

pub async fn populate_projects_table(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let project_ids: Vec<String> = conn
        .query(
            r"
            SELECT DISTINCT project_id FROM issues_master
            ",
        )
        .await?;

    for project_id in project_ids {
        let (repo_stars, project_logo): (i32, String) = conn
            .exec_first(
                r"
                SELECT repo_stars, repo_avatar FROM issues_open
                WHERE project_id = :project_id
                ",
                params! { "project_id" => &project_id },
            )
            .await?
            .unwrap_or((0, String::new())); // Default values if no matching row is found

        // Insert data into the projects table
        let query = r"
            INSERT INTO projects (project_id, repo_stars, project_logo)
            VALUES (:project_id, :repo_stars, :project_logo)
            ";

        conn.exec_drop(
            query,
            params! {
                "project_id" => &project_id,
                "repo_stars" => repo_stars,
                "project_logo" => &project_logo,
            },
        )
        .await?;
    }

    Ok(())
}

pub async fn consolidate_issues(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let select_query = r"
        SELECT 
            issues_open.issue_id, 
            issues_open.project_id, 
            issues_open.issue_title, 
            issues_open.issue_description, 
            issues_open.repo_stars, 
            issues_open.repo_avatar, 
            issues_closed.issue_assignees, 
            issues_closed.issue_linked_pr, 
            issues_comments.issue_status
        FROM issues_open
        LEFT JOIN issues_closed ON issues_open.issue_id = issues_closed.issue_id
        LEFT JOIN issues_comments ON issues_open.issue_id = issues_comments.issue_id";

    let result: Vec<(
        String,
        String,
        String,
        String,
        i32,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    )> = conn.query(select_query).await?;

    let mut transaction = conn
        .start_transaction(mysql_async::TxOpts::default())
        .await?;
    for row in result {
        transaction.exec_drop(
            r"
                INSERT INTO issues_master (issue_id, project_id, issue_title, issue_description, issue_assignees, issue_linked_pr, issue_status)
                VALUES (:issue_id, :project_id, :issue_title, :issue_description, :issue_assignees, :issue_linked_pr, :issue_status)",
            params! {
                "issue_id" => &row.0,
                "project_id" => &row.1,
                "issue_title" => &row.2,
                "issue_description" => &row.3,
                "issue_assignees" => row.6.as_deref(),
                "issue_linked_pr" => row.7.as_deref(),
                "issue_status" => row.8.as_deref(),
            },
        )
        .await?;
    }

    transaction.commit().await?;

    Ok(())
}

pub async fn add_mock_user(pool: &Pool, login_id: &str, email: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO participants (login_id, email  )
                  VALUES (:login_id, :email)";

    conn.exec_drop(
        query,
        params! {
            "login_id" => login_id,
            "email" => email,
        },
    )
    .await?;

    Ok(())
}

pub async fn add_issues_open(pool: &Pool, issue: IssueOpen) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_description, repo_stars, repo_avatar)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_description, :repo_stars, :repo_avatar)";

    conn.exec_drop(
        query,
        params! {
            "issue_id" => &issue.issue_id,
            "project_id" => &issue.project_id,
            "issue_title" => &issue.issue_title,
            "issue_description" => &issue.issue_description,
            "repo_stars" => issue.repo_stars ,
            "repo_avatar" => &issue.project_logo,
        },
    )
    .await?;

    Ok(())
}

pub async fn add_issues_closed(pool: &Pool, issue: IssueClosed) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let issue_assignees_json: Value = json!(issue.issue_assignees).into();

    let query = r"INSERT INTO issues_closed (issue_id, issue_assignees, issue_linked_pr)
                  VALUES (:issue_id, :issue_assignees, :issue_linked_pr)";

    conn.exec_drop(
        query,
        params! {
            "issue_id" => &issue.issue_id,
            "issue_assignees" => &issue_assignees_json,
            "issue_linked_pr" => issue.issue_linked_pr.as_deref(),
        },
    )
    .await?;

    Ok(())
}

pub async fn add_issues_comments(pool: &Pool, issue_comments: IssueComments) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    // let issue_status = todo!(comments);
    let issue_status = issue_comments
        .issue_comments
        .unwrap_or_default()
        .get(0)
        .unwrap_or(&"no comment obtained".to_string())
        .clone();

    let query = r"INSERT INTO issues_comments (issue_id, issue_status)
                  VALUES (:issue_id, :issue_status)";

    conn.exec_drop(
        query,
        params! {
            "issue_id" => &issue_comments.issue_id,
            "issue_status" => issue_status,
        },
    )
    .await?;

    Ok(())
}

pub async fn add_pull_request(pool: &Pool, pull: OuterPull) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let connected_issues_json: Value = json!(pull.connected_issues).into();

    let query = r"INSERT INTO pull_requests (pull_id, pull_title, pull_author, project_id, connected_issues, merged_by, pull_status)
                  VALUES (:pull_id, :pull_title, :pull_author, :project_id, :connected_issues, :merged_by, :pull_status)";

    conn.exec_drop(
        query,
        params! {
            "pull_id" => &pull.pull_id,
            "pull_title" => &pull.pull_title,
            "pull_author" => pull.pull_author.as_deref(),
            "project_id" => &pull.project_id,
            "connected_issues" => &connected_issues_json,
            "merged_by" => pull.merged_by.as_deref(),
            "pull_status" => "", // As pull_status is not provided in OuterPull, using empty string
        },
    )
    .await?;

    Ok(())
}

pub async fn table_open_comment_master(pool: &Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_master (
        issue_id, 
        project_id, 
        issue_title, 
        issue_description, 
        issue_status
    )
    SELECT 
        io.issue_id, 
        io.project_id, 
        io.issue_title, 
        io.issue_description, 
        ic.issue_status
    FROM 
        issues_open io
    JOIN 
        issues_comments ic ON io.issue_id = ic.issue_id  -- What do you mean by this line 
    ON DUPLICATE KEY UPDATE
        issue_status = VALUES(issue_status);";

    conn.query_drop(query).await?;

    Ok(())
}
