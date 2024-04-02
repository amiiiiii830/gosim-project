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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Project {
    pub project_id: String,
    pub project_logo: Option<String>,
    pub repo_stars: i32,
    pub project_description: Option<String>,
    pub issues_list: Option<Vec<String>>,
    pub issues_flagged: Option<Vec<String>>,
    pub participants_list: Option<Vec<String>>,
    pub total_budget_allocated: Option<i32>,
    pub total_budget_used: Option<i32>,
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

pub async fn project_exists(
    pool: &mysql_async::Pool,
    project_id: &str,
) -> Result<bool> {
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
                "SELECT project_id, project_logo, repo_stars, project_description, issues_list, issues_flagged, participants_list, total_budget_allocated, total_budget_used FROM projects ORDER BY project_id LIMIT {} OFFSET {}",
                page_size, offset
            ),
            |(project_id, project_logo, repo_stars, project_description, issues_list, issues_flagged, participants_list, total_budget_allocated, total_budget_used): (String, Option<String>, i32, Option<String>, Option<String>, Option<String>, Option<String>, Option<i32>, Option<i32>)| {
                Project {
                    project_id,
                    project_logo,
                    repo_stars,
                    project_description,
                    issues_list: issues_list.map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),
                    issues_flagged: issues_flagged.map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),
                    participants_list: participants_list.map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),       
                    total_budget_allocated,
                    total_budget_used
                }
            },
        )
        .await?;

    Ok(projects)
}
/* pub async fn issue_exists(
    pool: &mysql_async::Pool,
    issue_id: &str,
) -> Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<(i32,)> = conn
        .query_first(format!(
            "SELECT 1 FROM issues WHERE issue_id = '{}'",
            issue_id
        ))
        .await?;
    Ok(result.is_some())
} */

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

pub async fn approve_issue(
    pool: &mysql_async::Pool,
    issue_id: &str,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues 
                  SET issue_budget_approved = True, 
                      review_status = 'approve'
                  WHERE issue_id = :issue_id";

    let result = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await;

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

pub async fn conclude_issue_in_db(pool: &mysql_async::Pool, issue_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget_approved = True
                  WHERE issue_id = :issue_id";

    let result = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await;

    Ok(())
}

/* pub async fn pull_request_exists(pool: &Pool, pull_id: &str) -> Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<(i32,)> = conn
        .query_first(format!(
            "SELECT 1 FROM pull_requests WHERE pull_id = '{}'",
            pull_id
        ))
        .await?;
    Ok(result.is_some())
} */

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

pub async fn add_issues_open_batch(pool: &Pool, issues: Vec<IssueOpen>) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_description, repo_stars, repo_avatar)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_description, :repo_stars, :repo_avatar)";

    query
        .with(issues.iter().map(|issue| {
            params! {
                "issue_id" => &issue.issue_id,
                "project_id" => &issue.project_id,
                "issue_title" => &issue.issue_title,
                "issue_description" => &issue.issue_description,
                "repo_stars" => issue.repo_stars ,
                "repo_avatar" => &issue.project_logo,
            }
        }))
        .batch(&mut conn)
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

pub async fn open_comment_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO issues_master (
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
        issues_comments ic ON io.issue_id = ic.issue_id 
    ON DUPLICATE KEY UPDATE
        issue_status = VALUES(issue_status);";

    conn.query_drop(query).await?;

    Ok(())
}

pub async fn close_pull_master(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO issues_master (
        issue_id,
        issue_assignees,
        issue_linked_pr,
        project_id,
        issue_title,
        issue_description
    )
    SELECT 
        ic.issue_id, 
        ic.issue_assignees,
        ic.issue_linked_pr,
        im.project_id,  -- Get the project_id from issues_master
        im.issue_title,  -- Get the project_id from issues_master
        im.issue_description  -- Get the project_id from issues_master
    FROM 
        issues_closed ic
    JOIN 
        issues_master im ON ic.issue_id = im.issue_id
    ON DUPLICATE KEY UPDATE
        issue_assignees = VALUES(issue_assignees),
        issue_linked_pr = VALUES(issue_linked_pr);    
    ";

    conn.query_drop(query).await?;

    let query = r#"UPDATE issues_master AS im
    JOIN pull_requests AS pr
    ON JSON_CONTAINS(pr.connected_issues, CONCAT('"', im.issue_id, '"'), '$')
    SET
        im.issue_assignees = COALESCE(im.issue_assignees, JSON_ARRAY(pr.pull_author)),
        im.issue_linked_pr = COALESCE(im.issue_linked_pr, pr.pull_id)
    WHERE
        (im.issue_assignees IS NULL OR JSON_LENGTH(im.issue_assignees) = 0)  
        OR im.issue_linked_pr IS NULL;"#;

    conn.query_drop(query).await?;

    Ok(())
}

pub async fn open_master_project(pool: &mysql_async::Pool) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"
    INSERT INTO projects (
        project_id,
        project_logo,
        repo_stars,
        issues_list,
        issues_flagged
    )
    SELECT 
        im.project_id,
        io.repo_avatar AS project_logo,
        io.repo_stars,
        JSON_ARRAYAGG(im.issue_id) AS issues_list, 
        JSON_ARRAYAGG(CASE WHEN im.issue_status IS NOT NULL THEN im.issue_id ELSE NULL END) AS issues_flagged  
    FROM 
        issues_master im
    JOIN 
        issues_open io ON im.issue_id = io.issue_id
    GROUP BY 
        im.project_id, io.repo_avatar, io.repo_stars
    ON DUPLICATE KEY UPDATE
        issues_list = JSON_MERGE_PRESERVE(issues_list, VALUES(issues_list)),
        issues_flagged = JSON_MERGE_PRESERVE(issues_flagged, VALUES(issues_flagged));";

    conn.query_drop(query).await?;

    Ok(())
}
