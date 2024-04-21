use crate::{
    db_join::*, db_manipulate::*, db_populate::*, issue_bot::*, issue_tracker::*, llm_utils::*,
    vector_search::*,
};
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
    let _ = popuate_dbs(pool).await?;
    let _ = join_ops(pool).await?;
    let _ = cleanup_ops(pool).await?;
    // let _ = note_issues(pool).await?;
    let _ = summarize_issues(pool).await;
    let _ = populate_vector_db(pool).await;
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

    let query_comment =
        "label:hacktoberfest-accepted is:issue updated:>2024-01-01 -label:spam -label:invalid";
    log::info!("query_open: {:?}", query_open);

    let issue_comment_obj: Vec<IssueComment> = search_issues_comment(&query_comment).await?;
    let len = issue_comment_obj.len();
    log::info!("Issues comment recorded: {:?}", len);
    for issue in issue_comment_obj {
        let _ = add_issues_comment(pool, issue).await;
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

pub async fn populate_vector_db(pool: &Pool) -> anyhow::Result<()> {
    for item in get_issues_repos_from_db().await.expect("msg") {
        log::info!("uploading to vector_db: {:?}", item.0);
        let _ = upload_to_collection(&item.0, item.1.clone()).await;
        let _ = mark_id_indexed(&pool, &item.0).await;
        // match upload_to_collection(&item.0, item.1.clone()).await {
        //     Ok(_) => {
        //         mark_id_indexed(&pool, &item.0).await;
        //     }
        //     Err(e) => {
        //         log::error!("Error uploading to collection: {:?}", e);
        //     }
        // }
    }
    let _ = check_vector_db("gosim_search").await;

    Ok(())
}

pub async fn summarize_issues(pool: &Pool) -> anyhow::Result<()> {
    for issue in get_issues_from_db().await.expect("msg") {
        log::info!("{:?}", issue.0);
        let (issue_id, issue_title, issue_description, issue_assignees) = issue.clone();

        let parts: Vec<&str> = issue_id.split('/').collect();
        let owner = parts[3].to_string();
        let repo = parts[4].to_string();

        let system_prompt = r#"
        Summarize the GitHub issue in one paragraph without mentioning the issue number. Highlight the key problem and any signature information provided. The summary should be concise, informative, and easy to understand, prioritizing clarity and brevity. Additionally, extract high-level keywords that represent broader categories or themes relevant to the issue's purpose, features, and tools used. These keywords should help categorize the issue in a wider context and should not be too literal or specific.
        Expected Output:
        { \"summary\": \"the_summary_generated, a short paragraph summarizing the issue, including its purpose and features, without referencing the issue number.\",
          \"keywords\": \"keywords_list, a list of high-level keywords that encapsulate the broader context, categories, or themes of the issue, excluding specific details and issue numbers.\" }
        Ensure you reply in RFC8259-compliant JSON format."#;

        let assignees_str = match issue_assignees {
            Some(assignees) => {
                let assignees: Vec<&str> = assignees.split(',').collect();
                let assignees_str = assignees.join(", ");
                format!("assigned to: `{assignees_str}`")
            }
            None => String::from(""),
        };

        let raw_input_texts = if issue_description.len() < 200 {
            format!(
                "`{issue_title}` at repository `{repo}` by owner `{owner}`, {assignees_str} states: {issue_description}"
            )
        } else {
            format!(
                "Here is the input: The issue titled `{issue_title}` at repository `{repo}` by owner `{owner}` {assignees_str}, states in the body text: {issue_description}"
            ).chars().take(8000).collect::<String>()
        };
        let generated_summary = chat_inner_async(
            system_prompt,
            &raw_input_texts,
            200,
            "meta-llama/Meta-Llama-3-8B-Instruct",
        )
        .await?;

        let (summary, keyword_tags) = parse_summary_and_keywords(&generated_summary);
        let _ = add_summary_and_id(&pool, &issue.0, &summary, keyword_tags).await;
    }

    Ok(())
}
/* pub async fn summarize_project(pool: &Pool, repo_data: RepoData) -> anyhow::Result<()> {
    let parts: Vec<&str> = repo_data.project_id.split('/').collect();
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();

    let project_descrpition = repo_data.repo_description;
    let project_readme = repo_data.repo_readme;
    let main_language = repo_data.main_language;
    let system_prompt = r#"
    Summarize the GitHub repository's README or description in one paragraph. Highlight the project's key mission, tech stack, features, and essential tools used. The summary should be concise, informative, and easy to understand, prioritizing clarity and brevity. Additionally, extract keywords that represent the project's purpose, tech stack, features, and tools used. Ensure that the keywords list includes some of the same words used in the summary.
    Expected Output: 
    { \"summary\": \"the_summary_generated, a short paragraph summarizing the project, including its purpose, tech stack, and features.\", 
      \"keywords\": \"keywords_list, a list of keywords that represent the project's purpose, tech stack, features, and tools used.\" }
    ensure you reply in RFC8259-compliant JSON format."#;

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
    let summary = chat_inner_async(
        system_prompt,
        &raw_input_texts,
        200,
        "meta-llama/Meta-Llama-3-8B-Instruct",
    )
    .await?;

    let (summary, keyword_tags) = parse_summary_and_keywords(&summary);

    let _ = add_summary_and_id(&pool, &repo_data.project_id, &summary, keyword_tags).await;
    Ok(())
} */

pub async fn summarize_project_by_chained_chat(
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
        let model = "meta-llama/Meta-Llama-3-8B-Instruct";

        chat_inner_async(one_step_system_prompt, &raw_input_texts, 250, model).await?
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
            "chained_chat_to_sum_project",
        )
        .await?
    };
    let (summary, keyword_tags) = parse_summary_and_keywords(&generated_summary);

    let _ = add_summary_and_id(&pool, &repo_data.project_id, &summary, keyword_tags).await;
    Ok(())
}

pub async fn join_ops(pool: &Pool) -> anyhow::Result<()> {
    let _ = open_master(&pool).await?;
    let _ = assigned_master(&pool).await?;

    let _ = closed_master(&pool).await?;

    let _ = master_project(&pool).await?;
    let _ = sum_budget_to_project(&pool).await?;

    let query_repos: String = get_projects_as_repo_list(pool, 1).await?;

    let repo_data_vec: Vec<RepoData> = search_repos_in_batch(&query_repos).await?;

    for repo_data in repo_data_vec {
        let _ = fill_project_w_repo_data(&pool, repo_data.clone()).await?;
        let _ = summarize_project_by_chained_chat(&pool, repo_data).await?;
    }

    Ok(())
}

pub async fn cleanup_ops(pool: &Pool) -> anyhow::Result<()> {
    let _ = remove_pull_by_issued_linked_pr(&pool).await?;
    let _ = delete_issues_open_assigned_closed(&pool).await?;

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
