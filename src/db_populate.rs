use crate::db_manipulate::*;
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

pub async fn fill_project_w_repo_data(pool: &Pool, repo_data: RepoData) -> anyhow::Result<()> {
    let mut conn = pool.get_conn().await?;

    let project_id = repo_data.project_id;
    let project_logo = repo_data.project_logo;
    let repo_stars = repo_data.repo_stars;

    let project_description = if !repo_data.repo_readme.is_empty() {
        repo_data.repo_readme.clone()
    } else if !repo_data.repo_description.is_empty() {
        repo_data.repo_description.clone()
    } else {
        String::from("No description available")
    };

    match conn
        .exec_drop(
            r"UPDATE projects SET
            project_logo = :project_logo,
            repo_stars = :repo_stars,
            project_description = :project_description
        WHERE project_id = :project_id;",
            params! {
                "project_id" => project_id,
                "project_logo" => project_logo,
                "repo_stars" => repo_stars,
                "project_description" => project_description,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };
    Ok(())
}

pub async fn issue_exists(pool: &mysql_async::Pool, issue_id: &str) -> Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<(i32,)> = conn
        .query_first(format!(
            "SELECT 1 FROM issues WHERE issue_id = '{}'",
            issue_id
        ))
        .await?;
    Ok(result.is_some())
}

pub async fn pull_request_exists(pool: &Pool, pull_id: &str) -> Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<(i32,)> = conn
        .query_first(format!(
            "SELECT 1 FROM pull_requests WHERE pull_id = '{}'",
            pull_id
        ))
        .await?;
    Ok(result.is_some())
}

pub async fn add_issues_open(pool: &Pool, issue: IssueOpen) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_description)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_description)";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue.issue_id,
                "project_id" => &issue.project_id,
                "issue_title" => &issue.issue_title,
                "issue_description" => &issue.issue_description,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn add_issues_open_batch(pool: &Pool, issues: Vec<IssueOpen>) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_description)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_description)";

    match query
        .with(issues.iter().map(|issue| {
            params! {
                "issue_id" => &issue.issue_id,
                "project_id" => &issue.project_id,
                "issue_title" => &issue.issue_title,
                "issue_description" => &issue.issue_description,
            }
        }))
        .batch(&mut conn)
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn add_issues_closed(pool: &Pool, issue: IssueClosed) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let issue_assignees_json: Value = json!(issue.issue_assignees).into();

    let query = r"INSERT INTO issues_closed (issue_id, issue_assignees, issue_linked_pr)
                  VALUES (:issue_id, :issue_assignees, :issue_linked_pr)";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue.issue_id,
                "issue_assignees" => &issue_assignees_json,
                "issue_linked_pr" => issue.issue_linked_pr.as_deref(),
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn add_issues_assigned(pool: &Pool, issue_assigned: IssueAssigned) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_assigned (issue_id, issue_assignee, date_assigned)
                  VALUES (:issue_id, :issue_assignee, :date_assigned)";

    match conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue_assigned.issue_id,
                "issue_assignee" => &issue_assigned.issue_assignee,
                "date_assigned" => &issue_assigned.date_assigned,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}

pub async fn add_pull_request(pool: &Pool, pull: OuterPull) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let connected_issues_json: Value = json!(pull.connected_issues).into();

    let query = r"INSERT INTO pull_requests (pull_id, pull_title, pull_author, project_id, connected_issues, date_merged)
                  VALUES (:pull_id, :pull_title, :pull_author, :project_id, :connected_issues, :date_merged)";

    match conn
        .exec_drop(
            query,
            params! {
                "pull_id" => &pull.pull_id,
                "pull_title" => &pull.pull_title,
                "pull_author" => pull.pull_author.as_deref(),
                "project_id" => &pull.project_id,
                "connected_issues" => &connected_issues_json,
                "date_merged" => pull.merged_at,
            },
        )
        .await
    {
        Ok(_) => (),
        Err(e) => eprintln!("Error: {:?}", e),
    };

    Ok(())
}
