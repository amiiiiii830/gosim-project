//functions that changes entries in db
//entries are searched, changed according to rules when administrating over the records

use std::collections::HashMap;

use crate::db_populate::*;
use crate::{PREV_HOUR, TOTAL_BUDGET};
use anyhow::anyhow;
use chrono::{Duration, Utc};
use mysql_async::prelude::*;
use mysql_async::Row;
use mysql_async::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueSubset {
    pub issue_id: String,
    pub project_id: String,
    pub project_logo: String,
    pub issue_title: String,
    pub issue_creator: String,
    pub main_language: String,
    pub repo_stars: i32,
    pub issue_budget: Option<i32>,
    pub running_budget: (i32, i32, i32),
    pub issue_stats: (i32, i32, i32, i32),
    pub issue_status: Option<String>,
    pub review_status: String,
    #[serde(default = "default_value")]
    pub issue_budget_approved: bool,
}

fn default_value() -> bool {
    false
}

pub async fn batch_decline_issues_in_db(
    pool: &Pool,
    issue_ids: Vec<String>,
) -> anyhow::Result<(), String> {
    let mut conn = pool.get_conn().await.expect("Error getting connection");
    let mut failed_ids = Vec::new();
    for issue_id in issue_ids {
        if let Err(e) = conn
            .exec_drop(
                r"UPDATE issues_master SET review_status = 'decline',
                date_declined = NOW() 
                 WHERE issue_id = :issue_id",
                params! {
                    "issue_id" => &issue_id
                },
            )
            .await
        {
            failed_ids.push(issue_id);
            log::error!("Error batch decline issues: {:?}", e);
        }
    }

    if failed_ids.is_empty() {
        Ok(())
    } else {
        Err(failed_ids.join(","))
    }
}

pub async fn count_issues_by_status(pool: &Pool) -> anyhow::Result<(i32, i32, i32, i32)> {
    let mut conn = pool.get_conn().await?;
    let counts_query = format!(
        "SELECT
            (SELECT COUNT(*) FROM issues_master) as total_count,
            (SELECT COUNT(*) FROM issues_master WHERE review_status = 'approve') as approve_count,
            (SELECT COUNT(*) FROM issues_master WHERE review_status = 'decline') as decline_count"
    );

    let counts_rows: Vec<mysql_async::Row> = conn.query(counts_query).await?;
    let (total_count, approve_count, decline_count): (i32, i32, i32) = counts_rows
        .into_iter()
        .map(|row| {
            (
                row.get("total_count").unwrap_or_default(),
                row.get("approve_count").unwrap_or_default(),
                row.get("decline_count").unwrap_or_default(),
            )
        })
        .next() // There should be exactly one row
        .unwrap_or((0, 0, 0));

    let queue_count = total_count - (approve_count + decline_count);

    Ok((total_count, queue_count, approve_count, decline_count))
}

pub async fn count_budget_by_status(pool: &Pool) -> anyhow::Result<(i32, i32, i32)> {
    let mut conn = pool.get_conn().await?;
    let counts_query =
        "SELECT SUM(total_budget_allocated) as total_budget_allocated FROM projects;";

    let total_budget_allocated: i32 = conn
        .query_first(counts_query)
        .await?
        .unwrap_or(None) // Handle the case where no rows are returned
        .unwrap_or(0);

    let budget_balance = TOTAL_BUDGET - total_budget_allocated;

    Ok((TOTAL_BUDGET, total_budget_allocated, budget_balance))
}

fn build_query_clause(filters: Vec<&str>) -> String {
    let schema_array = [
        ("repo_stars", "repo_stars DESC"),
        ("issue_title", "issue_title ASC"),
        ("main_language", "main_language ASC"),
        ("issue_creator", "issue_creator ASC"),
        ("issue_budget", "issue_budget DESC"),
        ("issue_assignees", "issue_assignees ASC"),
        ("date_issue_assigned", "date_issue_assigned ASC"),
    ];

    let special_conditions = [
        ("main_language", "LENGTH(main_language) > 0"),
        ("issue_assignees", "issue_assignees IS NOT NULL"),
        ("queue", "review_status = 'queue'"),
        ("approve", "review_status = 'approve'"),
        ("decline", "review_status = 'decline'"),
    ];

    let schema_map: HashMap<&str, &str> = schema_array.into_iter().collect();
    let condition_map: HashMap<&str, &str> = special_conditions.into_iter().collect();

    let mut where_clause = String::new();
    let mut order_bys = Vec::new();

    for &filter in &filters {
        if let Some(&condition) = condition_map.get(filter) {
            where_clause = format!("WHERE {}", condition);
        } else if let Some(&order_by) = schema_map.get(filter) {
            order_bys.push(order_by);
        }
    }

    let order_by_clause = if order_bys.is_empty() {
        String::new()
    } else {
        format!("ORDER BY {}", order_bys.join(", "))
    };

    format!("{} {}", where_clause, order_by_clause)
        .trim()
        .to_string()
}

pub async fn list_issues_by_multi(
    pool: &Pool,
    filters: Vec<&str>,
    page: usize,
    page_size: usize,
) -> Result<Vec<IssueOut>> {
    let mut conn = pool.get_conn().await?;

    let (total_budget, total_budget_allocated, budget_balance) = count_budget_by_status(&pool)
        .await
        .expect("budget counting failure");

    let offset = (page - 1) * page_size;

    let filter_str = build_query_clause(filters);

    let query = format!(
        "SELECT issue_id, project_id, project_logo, issue_title, main_language, repo_stars, issue_budget, issue_creator, issue_description, issue_assignees, issue_linked_pr, issue_status, review_status, issue_budget_approved FROM issues_master {} LIMIT {} OFFSET {}",
        filter_str, page_size, offset
    );

    let rows: Vec<mysql_async::Row> = conn.query(query).await?;
    let (total_count, queue_count, approve_count, decline_count) = count_issues_by_status(&pool)
        .await
        .expect("failed to get issue stats");

    let mut issues = Vec::new();
    for row in rows {
        let issue = IssueOut {
            issue_id: row.get("issue_id").unwrap_or_default(),
            project_id: row.get("project_id").unwrap_or_default(),
            project_logo: row.get("project_logo").unwrap_or_default(),
            issue_title: row.get("issue_title").unwrap_or_default(),
            main_language: row.get("main_language").unwrap_or_default(),
            repo_stars: row.get::<i32, _>("repo_stars").unwrap_or_default(),
            issue_budget: row.get::<Option<i32>, _>("issue_budget").unwrap_or(None),
            issue_creator: row.get("issue_creator").unwrap_or_default(),
            issue_description: row.get("issue_description").unwrap_or_default(),
            issue_assignees: row
                .get::<Option<String>, _>("issue_assignees")
                .unwrap_or(None),
            issue_linked_pr: row
                .get::<Option<String>, _>("issue_linked_pr")
                .unwrap_or(None),
            issue_status: row.get::<Option<String>, _>("issue_status").unwrap_or(None),
            review_status: row.get("review_status").unwrap_or_default(),
            issue_budget_approved: row
                .get::<bool, _>("issue_budget_approved")
                .unwrap_or_default(),
            running_budget: (total_budget, total_budget_allocated, budget_balance),
            issue_stats: (total_count, queue_count, approve_count, decline_count),
        };

        issues.push(issue);
    }

    Ok(issues)
}

pub async fn list_issues_by_single(
    pool: &Pool,
    list_by: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<Vec<IssueSubset>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;

    let filter_str = match list_by {
        None => String::new(),
        Some(list_by) => {
            let list_by_str = list_by.to_string();
            build_query_clause(vec![&list_by_str])
        }
    };

    log::info!("filter_str: {:?}", filter_str);

    let (total_budget, total_budget_allocated, budget_balance) = count_budget_by_status(&pool)
        .await
        .expect("budget counting failure");

    let (total_count, queue_count, approve_count, decline_count) = count_issues_by_status(&pool)
        .await
        .expect("failed to get issue stats");
    let issues: Vec<IssueSubset> = conn
        .query_map(
            format!(
                "SELECT issue_id, project_id, project_logo, issue_title, main_language, repo_stars, issue_budget,issue_creator, issue_status, review_status, issue_budget_approved FROM issues_master {} LIMIT {} OFFSET {}",
                filter_str, page_size, offset
            ),
            |(issue_id, project_id, project_logo, issue_title, main_language, repo_stars, issue_budget, issue_creator, issue_status, review_status, issue_budget_approved): (String, String, String, String, String, i32, Option<i32>, String, Option<String>, Option<String>, Option<bool>)| {
                IssueSubset {
                    issue_id,
                    project_id,
                    project_logo,
                    issue_title,
                    main_language,
                    repo_stars,
                    issue_budget,
                    issue_creator,
                    issue_status,
                    review_status: review_status.unwrap_or_default(),
                    issue_budget_approved: issue_budget_approved.unwrap_or_default(),
                    running_budget: (total_budget, total_budget_allocated, budget_balance),
                    issue_stats: (total_count, queue_count, approve_count, decline_count),
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
            "SELECT project_id FROM projects WHERE project_logo is NULL ORDER BY project_id LIMIT :limit OFFSET :offset",
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

    let mut out = res.join(" ");
    out.push_str(" fork:true");
    Ok(out)
}

pub async fn get_updated_approved_issues_node_ids(pool: &Pool) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;

    let query = format!(
        "SELECT node_id FROM issues_master 
        WHERE review_status='approve' AND node_id IN (SELECT node_id FROM issues_updated) 
        ORDER BY issue_id ASC;"
    );

    let out: Vec<String> = conn.query_map(query, |node_id: String| node_id).await?;

    Ok(out)
}
/* pub async fn get_issues_open_from_master(
    pool: &Pool,
    page: u32,
) -> Result<Vec<IssueOpen>> {
    let page_size = 30u32;
    let offset = (page - 1) * page_size;
    let mut conn = pool.get_conn().await?;

    let query = format!(
        "SELECT issue_title, issue_id, issue_creator, issue_description, project_id FROM issues_master
        WHERE issue_id NOT IN (SELECT issue_or_project_id FROM issues_repos_summarized WHERE issue_or_project_summary IS NOT NULL)
        ORDER BY issue_id ASC
        LIMIT {} OFFSET {}",
        page_size, offset
    );

    let out: Vec<IssueOpen> = conn
        .query_map(
            query,
            |(issue_title, issue_id, issue_creator, issue_description, project_id): (
                String,
                String,
                String,
                String,
                String,
            )| IssueOpen {

                issue_title,
                issue_id,
                issue_creator,
                issue_budget: 0,
                issue_description,
                project_id,
            },
        )
        .await?;

    Ok(out)
} */

pub async fn list_projects_by(
    pool: &Pool,
    list_by: Option<&str>,
    page: usize,
    page_size: usize,
) -> Result<Vec<ProjectOut>> {
    let mut conn = pool.get_conn().await?;
    let offset = (page - 1) * page_size;

    let schema_array = [
        ("issues_count", "ORDER BY JSON_LENGTH(fp.issues_list) DESC"),
        (
            "total_budget_allocated",
            "ORDER BY fp.total_budget_allocated DESC",
        ),
        ("repo_stars", "ORDER BY fp.repo_stars DESC"),
        (
            "main_language",
            "WHERE LENGTH(fp.main_language) > 0 ORDER BY fp.main_language ASC",
        ),
    ];

    let schema_map: HashMap<&str, &str> = schema_array.into_iter().collect();

    let filter_str = match list_by {
        None => &"",
        Some(list_by) => schema_map.get(list_by).unwrap_or(&""),
    };

    let projects: Vec<ProjectOut> = conn
        .query_map(
            format!(
                "WITH FilteredProjects AS (
                SELECT 
                    project_id, 
                    project_logo, 
                    repo_stars, 
                    main_language, 
                    project_description, 
                    issues_list,   
                    total_budget_allocated
                FROM 
                    projects
            ),
            TotalCount AS (
                SELECT COUNT(*) AS total_count FROM FilteredProjects
            )
            SELECT 
                fp.project_id, 
                fp.project_logo, 
                fp.repo_stars, 
                fp.main_language, 
                fp.project_description, 
                fp.issues_list,   
                fp.total_budget_allocated,
                tc.total_count
            FROM 
                FilteredProjects fp, TotalCount tc
                            {}
            LIMIT {} OFFSET {}",
                filter_str, page_size, offset
            ),
            |(
                project_id,
                project_logo,
                repo_stars,
                main_language,
                project_description,
                issues_list,
                total_budget_allocated,
                total_count,
            ): (
                String,
                Option<String>,
                i32,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<i32>,
                i32,
            )| {
                ProjectOut {
                    project_id,
                    project_logo,
                    repo_stars,
                    main_language,
                    project_description,
                    issues_list: issues_list
                        .map_or(Some(Vec::new()), |s| serde_json::from_str(&s).ok()),
                    total_budget_allocated,
                    total_count,
                }
            },
        )
        .await?;

    Ok(projects)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueAndComments {
    pub issue_id: String,
    pub issue_title: String,
    pub project_id: String,
    pub main_language: String,
    pub repo_stars: i32,
    pub issue_creator: String,
    pub issue_description: String,
    pub issue_budget: Option<i32>,
    pub issue_assignees: Option<String>, // or a more specific type if you know the structure of the JSON
    pub issue_linked_pr: Option<String>,
    pub issue_status: Option<String>,
    pub review_status: String,
    pub issue_budget_approved: bool,
    pub issue_comments: Option<Vec<(String, String)>>,
}

pub async fn get_issue_w_comments_by_id(
    pool: &Pool,
    issue_id: &str,
) -> anyhow::Result<IssueAndComments> {
    let mut conn = pool.get_conn().await?;

    let issue_query = format!(
        "SELECT issue_id, project_id, main_language, repo_stars, issue_title, issue_creator, issue_description, issue_budget, issue_assignees, issue_linked_pr, issue_status, review_status, issue_budget_approved FROM issues_master WHERE issue_id = '{}'",
        issue_id
    );

    let comments_query = format!(
        "SELECT comment_creator, comment_body FROM issues_assign_comment WHERE issue_id = '{}' ORDER BY comment_date",
        issue_id
    );

    // Fetch the issue
    let issue_rows: Vec<mysql_async::Row> = conn.query(issue_query).await?;
    let issue_row = issue_rows
        .first()
        .ok_or_else(|| anyhow!("No issue found with the provided issue_id: {}", issue_id))
        .map_err(|_e| anyhow::anyhow!("discard mysql error: {_e}"))?;
    // .map_err(|e| mysql_async::Error::DriverError(mysql_async::DriverError::Other(format!("Failed to fetch issue: {}", e))))?;

    let issue = IssueOut {
        issue_id: issue_row.get("issue_id").unwrap_or_default(),
        project_id: issue_row.get("project_id").unwrap_or_default(),
        project_logo: issue_row.get("project_logo").unwrap_or_default(),
        main_language: issue_row.get("main_language").unwrap_or_default(),
        repo_stars: issue_row.get::<i32, _>("repo_stars").unwrap_or_default(),
        issue_title: issue_row.get("issue_title").unwrap_or_default(),
        issue_creator: issue_row.get("issue_creator").unwrap_or_default(),
        issue_description: issue_row.get("issue_description").unwrap_or_default(),
        issue_budget: issue_row
            .get::<Option<i32>, _>("issue_budget")
            .unwrap_or(None),
        issue_assignees: issue_row
            .get::<Option<String>, _>("issue_assignees")
            .unwrap_or(None),
        issue_linked_pr: issue_row
            .get::<Option<String>, _>("issue_linked_pr")
            .unwrap_or(None),
        issue_status: issue_row
            .get::<Option<String>, _>("issue_status")
            .unwrap_or(None),
        review_status: issue_row.get("review_status").unwrap_or_default(),
        issue_budget_approved: issue_row
            .get::<bool, _>("issue_budget_approved")
            .unwrap_or_default(),
        running_budget: (99999, 99999, 99999),
        issue_stats: (99999, 99999, 99999, 99999),
    };

    // Fetch the comments
    let comments_rows: Vec<mysql_async::Row> = conn.query(comments_query).await?;
    let comments: Vec<(String, String)> = comments_rows
        .into_iter()
        .map(|row| {
            (
                row.get("comment_creator").unwrap_or_default(),
                row.get("comment_body").unwrap_or_default(),
            )
        })
        .collect();

    Ok(IssueAndComments {
        issue_id: issue.issue_id,
        project_id: issue.project_id,
        issue_title: issue.issue_title,
        main_language: issue.main_language,
        repo_stars: issue.repo_stars,
        issue_creator: issue.issue_creator,
        issue_description: issue.issue_description,
        issue_budget: issue.issue_budget,
        issue_assignees: issue.issue_assignees,
        issue_linked_pr: issue.issue_linked_pr,
        issue_status: issue.issue_status,
        review_status: issue.review_status,
        issue_budget_approved: issue.issue_budget_approved,
        issue_comments: if comments.is_empty() {
            None
        } else {
            Some(comments)
        },
    })
}

pub async fn get_comments_by_issue_id(
    pool: &Pool,
    issue_id: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut conn = pool.get_conn().await?;

    let query_comments = r"SELECT comment_creator, comment_body FROM issues_assign_comment WHERE issue_id = :issue_id ORDER BY comment_date";

    match conn
        .exec(
            query_comments,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
    {
        Ok(ve) => {
            if !ve.is_empty() {
                Ok(ve
                    .into_iter()
                    .map(|(creator, body): (String, String)| (creator, body))
                    .collect::<Vec<(String, String)>>())
            } else {
                Err(anyhow::anyhow!("Error no comments found by issue_id"))
            }
        }
        Err(e) => {
            log::error!("Error getting comments by issue_id: {:?}", e);
            Err(anyhow::anyhow!(
                "Error getting comments found by issue_id: {:?}",
                e
            ))
        }
    }
}
pub async fn get_issue_ids_with_budget(pool: &Pool) -> Result<Vec<(String, i32)>> {
    let mut conn = pool.get_conn().await?;

    let selected_rows: Vec<(String,i32)> = conn
        .exec_map(
            "SELECT issue_id, issue_budget FROM issues_master WHERE issue_budget > 0 AND review_status='approve' AND date_approved > :one_hour_ago",
                        params! {
                "one_hour_ago" => &PREV_HOUR.to_string()
            },
            |(issue_id, issue_budget)| (issue_id, issue_budget)
        )
        .await?;
    Ok(selected_rows)
}

pub async fn get_issue_ids_declined(pool: &Pool) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;
    let selected_rows: Vec<String> = conn
        .query_map(
            "select issue_id from issues_master where review_status='decline' limit 50;",
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
            "SELECT issue_assignees, issue_id, issue_budget FROM issues_master WHERE issue_budget_approved=1 LIMIT 50",
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
    let _one_month_ago =
        (Utc::now() - Duration::try_days(30).unwrap().to_std().unwrap()).naive_utc();
    let formatted_one_month_ago = _one_month_ago.format("%Y-%m-%d %H:%M:%S").to_string();

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
) -> anyhow::Result<(), String> {
    let mut conn = pool.get_conn().await.expect("failed to get mysql pool");
    let exists_query = r"SELECT 1 FROM issues_master WHERE issue_id = :issue_id";
    let exists: Option<u8> = conn
        .exec_first(
            exists_query,
            params! {
                "issue_id" => issue_id,
            },
        )
        .await
        .map_err(|e| e.to_string())?;

    if exists.is_none() {
        return Err(format!("Issue with ID {} doesn't exist", issue_id));
    }

    let update_query = r"UPDATE issues_master 
                     SET issue_budget = :issue_budget, 
                         review_status = 'approve',
                         date_approved = NOW() 
                     WHERE issue_id = :issue_id";

    conn.exec_drop(
        update_query,
        params! {
            "issue_id" => issue_id,
            "issue_budget" => issue_budget,
        },
    )
    .await
    .map_err(|e| {
        log::error!("Error assigning issue budget: {:?}", e);
        format!("Error assigning issue budget: {:?}", e)
    })?;

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
                      review_status = 'decline',
                      date_declined = NOW() 
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
                  SET issue_budget_approved = True,
                  date_budget_approved = COALESCE(date_budget_approved, now())
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
        Ok(_) => {
            log::info!("Successfully concluded issue with ID: {}", issue_id);
        }
        Err(e) => {
            log::error!("Error concluding issue: {:?}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

pub async fn conclude_issues_batch_in_db(
    pool: &mysql_async::Pool,
    issue_ids: Vec<&str>,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_master 
                  SET issue_budget_approved = True,
                  date_budget_approved = COALESCE(date_budget_approved, now())
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

// pub async fn search_by_keyword_tags(tags_to_search: Vec<String>) -> anyhow::Result<Vec<String>> {
//     let mut conn = pool.get_conn().await?;

//     let query = r"select issue_or_project_id, keyword_tags from issues_repos_summarized
//                   WHERE :tags_to_search in keyword_tags";

//     for issue_id in issue_ids {
//         if let Err(e) = conn
//             .exec_drop(
//                 query,
//                 params! {
//                     "issue_id" => issue_id,
//                 },
//             )
//             .await
//         {
//             log::error!("Error concluding issues batch: {:?}", e);
//         };
//     }

//     Ok(())
// }

// pub async fn search_by_keyword_tags(pool: Pool, tags_to_search: Vec<String>) -> Result<Vec<String>> {
//     let mut conn = pool.get_conn().await?;
//     let mut results = Vec::new();
//     let mut unique_ids = std::collections::HashSet::new();

//     for tag in tags_to_search {
//         let tag_json = serde_json::to_string(&tag).map_err(|e| anyhow!("Failed to serialize tag: {:?}", e))?;
//         let query = r"SELECT issue_or_project_id, keyword_tags FROM issues_repos_summarized WHERE JSON_CONTAINS(keyword_tags, :tag)";

//         let rows: Vec<Row> = conn.exec(query, params! {
//             "tag" => tag_json,
//         }).await?;

//         for row in rows {
//             let issue_id: String = row.get("issue_or_project_id").unwrap();
//             // Ensure that each id is only added once
//             if unique_ids.insert(issue_id.clone()) {
//                 results.push(issue_id);
//             }
//         }
//     }

//     Ok(results)
// }

pub async fn search_by_keyword_tags(
    pool: Pool,
    tags_to_search: Vec<String>,
) -> Result<Vec<String>> {
    let mut conn = pool.get_conn().await?;
    let mut results = Vec::new();
    let mut unique_ids = std::collections::HashSet::new();

    let search_string = tags_to_search.join(" ");

    let query = r"SELECT issue_or_project_id FROM issues_repos_summarized WHERE MATCH(keyword_tags_text) AGAINST(:tags IN BOOLEAN MODE)";

    let rows: Vec<Row> = conn
        .exec(
            query,
            params! {
                "tags" => &search_string,
            },
        )
        .await?;

    for row in rows {
        let issue_id: String = row.get("issue_or_project_id").unwrap();
        if unique_ids.insert(issue_id.clone()) {
            results.push(issue_id);
        }
    }

    Ok(results)
}
