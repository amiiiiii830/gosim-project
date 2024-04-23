use crate::issue_tracker::*;
use crate::llm_utils::*;
use dotenv::dotenv;
use mysql_async::prelude::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueOut {
    pub issue_id: String,
    pub project_id: String,
    pub issue_title: String,
    pub issue_description: String,
    pub issue_budget: Option<i32>,
    pub issue_assignees: Option<Vec<String>>, // or a more specific type if you know the structure of the JSON
    pub issue_linked_pr: Option<String>,
    pub issue_status: Option<String>,
    pub review_status: String,
    #[serde(default = "default_value")]
    pub issue_budget_approved: bool,
}
fn default_value() -> bool {
    false
}

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
    pub total_budget_allocated: Option<i32>,
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

pub async fn project_exists(pool: &mysql_async::Pool, project_id: &str) -> anyhow::Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<u32> = conn
        .query_first(format!(
            "SELECT 1 FROM projects WHERE project_id = '{}'",
            project_id
        ))
        .await?;

    match result {
        Some(_) => Ok(true),
        None => {
            log::error!("Project not found");
            Err(anyhow::Error::msg("Project not found"))
        }
    }
}

pub async fn fill_project_w_repo_data(pool: &Pool, repo_data: RepoData) -> anyhow::Result<()> {
    let mut conn = pool.get_conn().await?;

    let project_id = repo_data.project_id;
    let project_logo = repo_data.project_logo;
    let repo_stars = repo_data.repo_stars;

    let project_description = if !repo_data.repo_description.is_empty() {
        repo_data.repo_description.clone()
    } else if !repo_data.repo_readme.is_empty() {
        repo_data.repo_readme.chars().take(1000).collect()
    } else {
        String::from("No description available")
    };

    if let Err(e) = conn
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
        log::error!("Failed to fill project with repo data: {:?}", e);
    };
    Ok(())
}

pub async fn issue_exists(pool: &mysql_async::Pool, issue_id: &str) -> anyhow::Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<u32> = conn
        .query_first(format!(
            "SELECT 1 FROM issues WHERE issue_id = '{}'",
            issue_id
        ))
        .await?;

    match result {
        Some(_) => Ok(true),
        None => {
            log::error!("Issue not found");
            Err(anyhow::Error::msg("Issue not found"))
        }
    }
}

pub async fn pull_request_exists(pool: &mysql_async::Pool, pull_id: &str) -> anyhow::Result<bool> {
    let mut conn = pool.get_conn().await?;
    let result: Option<u32> = conn
        .query_first(format!(
            "SELECT 1 FROM pull_requests WHERE pull_id = '{}'",
            pull_id
        ))
        .await?;

    match result {
        Some(_) => Ok(true),
        None => {
            log::error!("Pull request not found");
            Err(anyhow::Error::msg("Pull request not found"))
        }
    }
}

pub async fn add_issues_open(pool: &Pool, issue: &IssueOpen) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_creator, issue_budget, issue_description)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_creator, :issue_budget, :issue_description)";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue.issue_id,
                "project_id" => &issue.project_id,
                "issue_title" => &issue.issue_title,
                "issue_creator" => &issue.issue_creator,
                "issue_budget" => &issue.issue_budget,
                "issue_description" => &issue.issue_description,
            },
        )
        .await
    {
        // log::error!("Error add issues_open: {:?}", e);
    };

    Ok(())
}

pub async fn add_issues_comment(pool: &Pool, issue: IssueComment) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_comment (issue_id, comment_creator, comment_date, comment_body)
    SELECT :issue_id, :comment_creator, :comment_date, :comment_body
    FROM dual
    WHERE NOT EXISTS (
        SELECT 1 FROM issues_comment
        WHERE issue_id = :issue_id AND comment_date = :comment_date
    ) LIMIT 1;";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue.issue_id,
                "comment_creator" => &issue.comment_creator,
                "comment_date" => &issue.comment_date,
                "comment_body" => &issue.comment_body,
            },
        )
        .await
    {
        if let mysql_async::Error::Server(server_error) = &e {
            if server_error.code == 23000 {
                log::info!("Skipping duplicate comment: {:?}", issue);
                return Ok(());
            }
        }
        log::error!("Error add issues_comment: {:?}", e);
        return Err(e.into());
    }

    Ok(())
}
pub async fn add_issues_open_batch(pool: &Pool, issues: Vec<IssueOpen>) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO issues_open (issue_id, project_id, issue_title, issue_budget, issue_description)
                  VALUES (:issue_id, :project_id, :issue_title, :issue_budget, :issue_description)";

    if let Err(e) = query
        .with(issues.iter().map(|issue| {
            params! {
                "issue_id" => &issue.issue_id,
                "project_id" => &issue.project_id,
                "issue_title" => &issue.issue_title,
                "issue_budget" => &issue.issue_budget,
                "issue_description" => &issue.issue_description,
            }
        }))
        .batch(&mut conn)
        .await
    {
        log::error!("Error add issues_open in batch: {:?}", e);
    };

    Ok(())
}

pub async fn add_issues_closed(pool: &Pool, issue: IssueClosed) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let issue_assignees_json: Value = json!(issue.issue_assignees).into();

    let query = r"INSERT INTO issues_closed (issue_id, issue_assignees, issue_linked_pr)
                  VALUES (:issue_id, :issue_assignees, :issue_linked_pr)";

    if let Err(e) = conn
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
        log::error!("Error add issues_closed: {:?}", e);
    };

    Ok(())
}

pub async fn add_issues_assigned(pool: &Pool, issue_assigned: IssueAssigned) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let issue_assignee = if issue_assigned.issue_assignee.is_empty() {
        None
    } else {
        Some(issue_assigned.issue_assignee)
    };

    let query = r"INSERT INTO issues_assigned (issue_id, issue_assignee, date_assigned)
                  VALUES (:issue_id, :issue_assignee, :date_assigned)";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_id" => &issue_assigned.issue_id,
                "issue_assignee" => &issue_assignee,
                "date_assigned" => &issue_assigned.date_assigned,
            },
        )
        .await
    {
        log::error!("Error add issues_assigned: {:?}", e);
    };

    Ok(())
}

pub async fn mark_id_indexed(pool: &Pool, issue_or_project_id: &str) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"UPDATE issues_repos_summarized
    SET indexed=1 WHERE issue_or_project_id = :issue_or_project_id";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_or_project_id" => &issue_or_project_id,
            },
        )
        .await
    {
        log::error!("Error marking issue_or_project_id: {:?}", e);
    };

    Ok(())
}

pub async fn add_or_update_summary_and_id(
    pool: &Pool,
    issue_or_project_id: &str,
    issue_or_project_summary: &str,
    keyword_tags: Vec<String>,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;
    let keyword_tags_json_str = json!(keyword_tags).to_string();

    let query = r"INSERT INTO issues_repos_summarized (issue_or_project_id, issue_or_project_summary, keyword_tags)
                  VALUES (:issue_or_project_id, :issue_or_project_summary, :keyword_tags_json_str)
                  ON DUPLICATE KEY UPDATE
                  keyword_tags = IF(JSON_LENGTH(keyword_tags) = 0 AND LENGTH(:keyword_tags_json_str) > 0, :keyword_tags_json_str, keyword_tags)";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_or_project_id" => &issue_or_project_id,
                "issue_or_project_summary" => &issue_or_project_summary,
                "keyword_tags_json_str" => &keyword_tags_json_str,
            },
        )
        .await
    {
        // Log the error if the query fails
        log::error!("Error adding or updating issue_or_project_id: {:?}", e);
        return Err(e);
    };

    Ok(())
}
/* pub async fn add_summary_and_id(
    pool: &Pool,
    issue_or_project_id: &str,
    issue_or_project_summary: &str,
    keyword_tags: Vec<String>,
) -> Result<()> {
    let mut conn = pool.get_conn().await?;
    let keyword_tags_json_str = json!(keyword_tags).to_string();
    let query = r"INSERT INTO issues_repos_summarized (issue_or_project_id, issue_or_project_summary, keyword_tags)
                  VALUES (:issue_or_project_id, :issue_or_project_summary, :keyword_tags_json_str)";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "issue_or_project_id" => &issue_or_project_id,
                "issue_or_project_summary" => &issue_or_project_summary,
                "keyword_tags_json_str" => &keyword_tags_json_str, // Corrected parameter name here
            },
        )
        .await
    {
        // log::error!("Error adding issue_or_project_id: {:?}", e);
    };

    Ok(())
} */

pub async fn add_pull_request(pool: &Pool, pull: OuterPull) -> Result<()> {
    let mut conn = pool.get_conn().await?;

    let query = r"INSERT INTO pull_requests (pull_id, pull_title, pull_author, project_id, date_merged)
                  VALUES (:pull_id, :pull_title, :pull_author, :project_id, :date_merged)";

    if let Err(e) = conn
        .exec_drop(
            query,
            params! {
                "pull_id" => &pull.pull_id,
                "pull_title" => &pull.pull_title,
                "pull_author" => pull.pull_author.as_deref(),
                "project_id" => &pull.project_id,
                "date_merged" => pull.merged_at,
            },
        )
        .await
    {
        log::error!("Error add pull_request: {:?}", e);
    };

    Ok(())
}

pub async fn get_issues_repos_from_db() -> Result<Vec<(String, String)>> {
    let pool = get_pool().await;
    let mut conn = pool.get_conn().await?;

    let query = r"SELECT issue_or_project_id, issue_or_project_summary FROM issues_repos_summarized WHERE indexed=0 limit 50";

    let entries: Vec<(String, String)> = conn
        .query_map(
            query,
            |(issue_or_project_id, issue_or_project_summary): (String, String)| {
                (issue_or_project_id, issue_or_project_summary)
            },
        )
        .await?;

    Ok(entries)
}

pub async fn get_issues_from_db() -> Result<Vec<(String, String, String, Option<String>)>> {
    let pool = get_pool().await;
    let mut conn = pool.get_conn().await?;

    let query = r"SELECT issue_id, issue_title, issue_description, issue_assignees FROM issues_master WHERE issue_id not in (SELECT issue_or_project_id FROM issues_repos_summarized) limit 50";

    let issues: Vec<(String, String, String, Option<String>)> = conn
        .query_map(
            query,
            |(issue_id, issue_title, issue_description, issue_assignees): (
                String,
                String,
                String,
                Option<String>,
            )| { (issue_id, issue_title, issue_description, issue_assignees) },
        )
        .await?;

    Ok(issues)
}

pub async fn summarize_issue_add_in_db(pool: &Pool, issue: &IssueOpen) -> anyhow::Result<()> {
    let issue_clone = issue.clone();
    let issue_title = issue_clone.issue_title;
    let issue_id = issue_clone.issue_id;
    let issue_description = issue_clone.issue_description;

    let parts: Vec<&str> = issue_id.split('/').collect();
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();

    let system_prompt = r#"
        Summarize the GitHub issue in one paragraph without mentioning the issue number. Highlight the key problem and any signature information provided. The summary should be concise, informative, and easy to understand, prioritizing clarity and brevity. Additionally, extract high-level keywords that represent broader categories or themes relevant to the issue's purpose, features, and tools used. These keywords should help categorize the issue in a wider context and should not be too literal or specific, avoiding overly long phrases unless absolutely necessary.
        Expected Output:
        { \"summary\": \"the_summary_generated, a short paragraph summarizing the issue, including its purpose and features, without referencing the issue number.\",
          \"keywords\": [\"keywords_list, a list of high-level keywords that encapsulate the broader context, categories, or themes of the issue, excluding specific details and issue numbers.\"] }
        Ensure you reply in RFC8259-compliant JSON format."#;

    let raw_input_texts = if issue_description.len() < 200 {
        format!(
                "`{issue_title}` at repository `{repo}` by owner `{owner}`, states: {issue_description}"
            )
    } else {
        format!(
                "Here is the input: The issue titled `{issue_title}` at repository `{repo}` by owner `{owner}`, states in the body text: {issue_description}"
            ).chars().take(8000).collect::<String>()
    };
    let generated_summary = chat_inner_async(system_prompt, &raw_input_texts, 200).await?;

    let (summary, keyword_tags) = parse_summary_and_keywords(&generated_summary);
    log::info!("{}, {:?}", issue_id, keyword_tags.clone());
    let _ = add_or_update_summary_and_id(&pool, &issue_id, &summary, keyword_tags).await;

    Ok(())
}
pub async fn summarize_project_add_in_db_one_step(
    pool: &Pool,
    repo_data: RepoData,
) -> anyhow::Result<()> {
    let parts: Vec<&str> = repo_data.project_id.split('/').collect();
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();

    let project_descrpition = repo_data.repo_description;
    let project_readme = repo_data.repo_readme;
    let main_language = repo_data.main_language;
    let system_prompt = r#"
    Summarize the GitHub repository's README or description in one paragraph. Highlight the project's key mission, tech stack, features, and essential tools used. The summary should be concise, informative, and easy to understand, prioritizing clarity and brevity. Focus on the core technological aspects and user benefits, excluding operational details like a project's upkeeping information or procedural guidelines. Additionally, extract high-level keywords that represent broader categories or themes relevant to the project's purpose, tech stack, features, and tools used. These keywords should help categorize the project in a wider context and should not be too literal or specific. Ensure that the keywords list includes broad terms that encapsulate the project's overarching themes and are reflective of the words used in the summary. 
    Expected Output: 
    { \"summary\": \"the_summary_generated, a short paragraph summarizing the project, including its purpose, tech stack, and features.\", 
      \"keywords\": \"keywords_list, a list of high-level keywords that encapsulate the broader context, categories, or themes of the project, excluding specific details.\" },
    ensure you reply in RFC8259-compliant JSON format.
    "#;

    let use_lang_str = if main_language.is_empty() {
        String::from("")
    } else {
        format!("mainly uses `{main_language}` in the project")
    };

    let project_readme_str = match project_readme.is_empty() {
        false => format!("states in readme: {project_readme}"),
        true => String::from(""),
    };

    let raw_input_texts = if project_readme.len() < 200 {
        format!(
            "The repository `{repo}` by owner `{owner}` {use_lang_str},`{project_descrpition}`, {project_readme_str}"
        )
    } else {
        format!(
                "Here is the input: The repository `{repo}`  by owner `{owner}` {use_lang_str}, has a short text description: `{project_descrpition}`, mentioned more details in readme: `{project_readme}`"
            ).chars().take(8000).collect::<String>()
    };

    let generated_summary = chat_inner_async(system_prompt, &raw_input_texts, 250).await?;
    let (summary, keyword_tags) = parse_summary_and_keywords(&generated_summary);

    let _ =
        add_or_update_summary_and_id(&pool, &repo_data.project_id, &summary, keyword_tags).await;
    Ok(())
}

pub async fn summarize_project_add_in_db(pool: &Pool, repo_data: RepoData) -> anyhow::Result<()> {
    let parts: Vec<&str> = repo_data.project_id.split('/').collect();
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();

    let project_descrpition = repo_data.repo_description;
    let project_readme = repo_data.repo_readme;
    let main_language = repo_data.main_language;
    let system_prompt = r#"
    Summarize the GitHub repository's README or description in one paragraph. Highlight the project's key mission, tech stack, features, and essential tools used. The summary should be concise, informative, and easy to understand, prioritizing clarity and brevity. Focus on the core technological aspects and user benefits, excluding operational details like a project's upkeeping information or procedural guidelines. Additionally, extract high-level keywords that represent broader categories or themes relevant to the project's purpose, tech stack, features, and tools used. These keywords should help categorize the project in a wider context and should not be too literal or specific. Ensure that the keywords list includes broad terms that encapsulate the project's overarching themes and are reflective of the words used in the summary. 
    Expected Output: 
    - A short paragraph summarizing the project, including its purpose, tech stack, and features. 
    - A list of high-level keywords that encapsulate the broader context, categories, or themes of the project, excluding specific details. Reply in JSON format:
    { \"summary\": \"the_summary_generated\", 
        \"keywords\": \"keywords_list\" }
    "#;

    let usr_prompt_2 = r#"
    fit the information you received into a RFC8259-compliant JSON:
```json
    { 
        \"summary\": \"the_summary_generated\", 
        \"keywords\": \"keywords_list\"
    }
    ```
    Ensure that the JSON is properly formatted, with correct escaping of special characters. Avoid adding any non-JSON content or formatting    
    "#;

    let use_lang_str = if main_language.is_empty() {
        String::from("")
    } else {
        format!("mainly uses `{main_language}` in the project")
    };

    let project_readme_str = match project_readme.is_empty() {
        false => format!("states in readme: {project_readme}"),
        true => String::from(""),
    };

    let generated_summary = if project_readme.len() < 200 {
        let raw_input_texts = format!(
            "The repository `{repo}` by owner `{owner}` {use_lang_str},`{project_descrpition}`, {project_readme_str}"
        );

        let one_step_system_prompt = r#"Summarize the GitHub repository's README or description in one paragraph. Extract high-level keywords that represent broader categories or themes relevant to the project's purpose, technologies, features, and tools used. Infer plausible details based on common patterns or typical project characteristics related to the technologies or themes mentioned. These keywords should help categorize the project in a wider context and should not be too literal or specific. Expected Output: { \"summary\": \"the_summary_generated\", \"keywords\": \"keywords_list\" }, ensure you reply in RFC8259-compliant JSON format."#;

        chat_inner_async(one_step_system_prompt, &raw_input_texts, 250).await?
    } else {
        let raw_input_texts = format!(
                "Here is the input: The repository `{repo}`  by owner `{owner}` {use_lang_str}, has a short text description: `{project_descrpition}`, mentioned more details in readme: `{project_readme}`"
            ).chars().take(8000).collect::<String>();

        chain_of_chat(
            &system_prompt,
            &raw_input_texts,
            "chat_id_chain_chat",
            400,
            &usr_prompt_2,
            200,
        )
        .await?
    };
    let (summary, keyword_tags) = parse_summary_and_keywords(&generated_summary);

    let _ =
        add_or_update_summary_and_id(&pool, &repo_data.project_id, &summary, keyword_tags).await;
    Ok(())
}
