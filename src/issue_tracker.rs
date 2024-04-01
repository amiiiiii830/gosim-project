use anyhow::anyhow;
use http_req::{
    request::{Method, Request},
    uri::Uri,
};

use serde::{Deserialize, Serialize};
use std::env;

pub async fn github_http_post_gql(query: &str) -> anyhow::Result<Vec<u8>> {
    let token = env::var("GITHUB_TOKEN").expect("github_token is required");
    let base_url = "https://api.github.com/graphql";
    let base_url = Uri::try_from(base_url).unwrap();
    let mut writer = Vec::new();

    let query = serde_json::json!({"query": query});
    match Request::new(&base_url)
        .method(Method::POST)
        .header("User-Agent", "flows-network connector")
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Length", &query.to_string().len())
        .body(&query.to_string().into_bytes())
        .send(&mut writer)
    {
        Ok(res) => {
            if !res.status_code().is_success() {
                log::error!("Github http error {:?}", res.status_code());
                return Err(anyhow::anyhow!("Github http error {:?}", res.status_code()));
            }
            Ok(writer)
        }
        Err(_e) => {
            log::error!("Error getting response from Github: {:?}", _e);
            Err(anyhow::anyhow!(_e))
        }
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueComments {
    pub issue_id: String, // url of an issue
    pub issue_title: String,
    pub issue_body: Option<String>,
    pub issue_assignees: Option<Vec<String>>,
    pub issue_comments: Option<Vec<String>>,
}

#[allow(non_snake_case)]
pub async fn search_issues_w_update_comments(query: &str) -> anyhow::Result<Vec<IssueComments>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        issueCount: Option<i32>,
        nodes: Option<Vec<IssueNode>>,
        pageInfo: Option<PageInfo>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PageInfo {
        endCursor: Option<String>,
        hasNextPage: bool,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct IssueNode {
        url: Option<String>,
        title: Option<String>,
        body: Option<String>,
        assignees: Option<AssigneeNodes>,
        comments: Option<Comments>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct AssigneeNodes {
        nodes: Option<Vec<Assignee>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Assignee {
        name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Comments {
        nodes: Option<Vec<Comment>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Comment {
        author: Option<Author>,
        body: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..10 {
        let query_str = format!(
            r#"
                query {{
                    search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                        issueCount
                        nodes {{
                            ... on Issue {{
                                url
                                body
                                assignees(first: 5) {{
                                    nodes {{
                                        name
                                    }}
                                }}
                                comments(first: 50) {{
                                nodes {{
                                    author {{
                                        login
                                    }}
                                    body
                                }}
                                }}
                            }}
                        }}
                        pageInfo {{
                            endCursor
                            hasNextPage
                        }}
                    }}
                }}
                "#,
            query.replace("\"", "\\\""),
            after_cursor
                .as_ref()
                .map_or(String::from("null"), |c| format!("\"{}\"", c)),
        );

        let response_body = github_http_post_gql(&query_str)
            .await
            .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

        let response: GraphQLResponse = serde_json::from_slice(&response_body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let temp_str = String::from("");
                        let comments = issue.comments.map_or(Vec::new(), |comments| {
                            comments.nodes.map_or(Vec::new(), |nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|comment| {
                                        Some(format!(
                                            "{} commented: `{}`",
                                            comment.author.as_ref().map_or("", |a| a
                                                .login
                                                .as_ref()
                                                .unwrap_or(&temp_str)),
                                            comment.body.as_ref().unwrap_or(&temp_str)
                                        ))
                                    })
                                    .collect()
                            })
                        });
                        let assignees = issue.assignees.as_ref().map_or(Vec::new(), |assignees| {
                            assignees.nodes.as_ref().map_or(Vec::new(), |nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|assignee| assignee.name.clone())
                                    .collect()
                            })
                        });

                        // let comments_summary = check_comments().await;
                        let _comments_summary = String::from("placeholder");

                        all_issues.push(IssueComments {
                            issue_id: issue.url.unwrap_or_default(), // Assuming issue.url is the issue_id
                            issue_title: issue.title.unwrap_or_default(),
                            issue_assignees: Some(assignees),
                            issue_body: issue.body.clone(),
                            issue_comments: Some(comments),
                        });
                    }
                }

                if let Some(page_info) = search.pageInfo {
                    if page_info.hasNextPage {
                        after_cursor = page_info.endCursor
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(all_issues)
}

pub async fn check_comments() -> String {
    let mut _comment_summary = String::new();
    todo!("Implement this function");

    _comment_summary
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueOpen {
    pub issue_title: String,
    pub issue_id: String,          // url of an issue
    pub issue_description: String, // description of the issue, could be truncated body text
    pub project_id: String,        // url of the repo
    pub repo_stars: i64,
    pub project_logo: String, // repo owner avatar
}

#[allow(non_snake_case)]
pub async fn search_issues_open(query: &str) -> anyhow::Result<Vec<IssueOpen>> {
    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        issueCount: Option<i32>,
        nodes: Option<Vec<Issue>>,
        pageInfo: Option<PageInfo>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PageInfo {
        endCursor: Option<String>,
        hasNextPage: bool,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Issue {
        title: String,
        url: String,
        body: Option<String>,
        author: Option<Author>,
        repository: Option<Repository>,
        labels: Option<LabelNodes>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Repository {
        url: Option<String>,
        stargazers: Option<Stargazers>,
        owner: Option<Owner>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Owner {
        avatarUrl: Option<String>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Stargazers {
        totalCount: Option<i64>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct LabelNodes {
        nodes: Option<Vec<Label>>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Label {
        name: Option<String>,
    }
    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..10 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                            ... on Issue {{
                                title
                                url
                                body
                                author {{
                                    login
                                }}
                                repository {{
                                    url
                                    stargazers {{
                                        totalCount
                                    }}
                                    owner {{
                                        avatarUrl
                                    }}
                                }}
                                labels(first: 10) {{
                                    nodes {{
                                        name
                                    }}
                                }}
                            }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
            "#,
            query.replace("\"", "\\\""),
            after_cursor
                .as_ref()
                .map_or(String::from("null"), |c| format!("\"{}\"", c)),
        );

        let response_body = github_http_post_gql(&query_str)
            .await
            .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

        let response: GraphQLResponse = serde_json::from_slice(&response_body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let _labels = issue.labels.as_ref().map_or(Vec::new(), |labels| {
                            labels.nodes.as_ref().map_or(Vec::new(), |nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|label| label.name.clone())
                                    .collect()
                            })
                        });

                        let body = issue
                            .body
                            .clone()
                            .unwrap_or_default()
                            .chars()
                            .take(240)
                            .collect();
                        all_issues.push(IssueOpen {
                            issue_title: issue.title,
                            issue_id: issue.url, // Assuming issue.url is the issue_id
                            issue_description: body,
                            project_id: issue
                                .repository
                                .as_ref()
                                .and_then(|repo| repo.url.clone())
                                .unwrap_or_default(),
                            repo_stars: issue
                                .repository
                                .as_ref()
                                .and_then(|repo| {
                                    repo.stargazers
                                        .as_ref()
                                        .and_then(|star| star.totalCount.clone())
                                })
                                .unwrap_or_default(),

                            project_logo: issue
                                .repository
                                .as_ref()
                                .and_then(|repo| {
                                    repo.owner
                                        .as_ref()
                                        .and_then(|owner| owner.avatarUrl.clone())
                                })
                                .unwrap_or_default(),
                        });
                    }
                }

                if let Some(pageInfo) = search.pageInfo {
                    if pageInfo.hasNextPage {
                        after_cursor = pageInfo.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(all_issues)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueClosed {
    pub issue_id: String, // url of an issue
    pub issue_labels: Vec<String>,
    pub issue_assignees: Option<Vec<String>>,
    pub issue_linked_pr: Option<String>,
    pub linked_pr_title: Option<String>,
    pub close_reason: Option<String>,
    pub closer_login: Option<String>,
    pub issue_status: Option<String>,
}

#[allow(non_snake_case)]
pub async fn search_issues_closed(query: &str) -> anyhow::Result<Vec<IssueClosed>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        issueCount: Option<i32>,
        nodes: Option<Vec<Issue>>,
        pageInfo: Option<PageInfo>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PageInfo {
        endCursor: Option<String>,
        hasNextPage: bool,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Issue {
        url: Option<String>,
        labels: Option<LabelNodes>,
        assignees: Option<AssigneeNodes>,
        timelineItems: Option<TimelineItems>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct LabelNodes {
        nodes: Option<Vec<Label>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Label {
        name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct AssigneeNodes {
        nodes: Option<Vec<Assignee>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Assignee {
        name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct TimelineItems {
        nodes: Option<Vec<ClosedEvent>>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct ClosedEvent {
        stateReason: Option<String>,
        closer: Option<Closer>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Closer {
        title: Option<String>,
        url: Option<String>,
        author: Option<Author>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..10 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                        ... on Issue {{
                            url
                            labels(first: 10) {{
                                nodes {{
                                    name
                                }}
                            }}
                            assignees(first: 5) {{
                                nodes {{
                                    name
                                }}
                            }}
                            timelineItems(first: 1, itemTypes: [CLOSED_EVENT]) {{
                                nodes {{
                                    ... on ClosedEvent {{
                                        stateReason
                                        closer {{
                                            ... on PullRequest {{
                                                title
                                                url
                                                author {{
                                                    login
                                                }}
                                            }}
                                        }}
                                    }}
                                }}
                            }}
                        }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
            "#,
            query.replace("\"", "\\\""),
            after_cursor
                .as_ref()
                .map_or(String::from("null"), |c| format!("\"{}\"", c)),
        );

        let response_body = github_http_post_gql(&query_str)
            .await
            .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

        let response: GraphQLResponse = serde_json::from_slice(&response_body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let issue_labels = issue.labels.as_ref().map_or(Vec::new(), |labels| {
                            labels.nodes.as_ref().map_or(Vec::new(), |nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|label| label.name.clone())
                                    .collect()
                            })
                        });

                        let issue_assignees =
                            issue.assignees.as_ref().map_or(Vec::new(), |assignees| {
                                assignees.nodes.as_ref().map_or(Vec::new(), |nodes| {
                                    nodes
                                        .iter()
                                        .filter_map(|assignee| assignee.name.clone())
                                        .collect()
                                })
                            });

                        let (close_reason, close_pull_request, close_pr_title, closer_login) =
                            issue.timelineItems.as_ref().map_or(
                                (String::new(), String::new(), String::new(), String::new()),
                                |items| {
                                    items.nodes.as_ref().map_or(
                                        (
                                            String::new(),
                                            String::new(),
                                            String::new(),
                                            String::new(),
                                        ),
                                        |nodes| {
                                            nodes
                                                .iter()
                                                .filter_map(|event| {
                                                    if let Some(closer) = &event.closer {
                                                        Some((
                                                            event
                                                                .stateReason
                                                                .clone()
                                                                .unwrap_or_default(),
                                                            closer.url.clone().unwrap_or_default(),
                                                            closer
                                                                .title
                                                                .clone()
                                                                .unwrap_or_default(),
                                                            closer.author.as_ref().map_or(
                                                                String::new(),
                                                                |author| {
                                                                    author
                                                                        .login
                                                                        .clone()
                                                                        .unwrap_or_default()
                                                                },
                                                            ),
                                                        ))
                                                    } else {
                                                        Some((
                                                            String::new(),
                                                            String::new(),
                                                            String::new(),
                                                            String::new(),
                                                        ))
                                                    }
                                                })
                                                .next()
                                                .unwrap_or((
                                                    String::new(),
                                                    String::new(),
                                                    String::new(),
                                                    String::new(),
                                                ))
                                        },
                                    )
                                },
                            );

                        let issue_id = issue.url.clone().unwrap_or_default();

                        let potential_problems_summary = issue_checker(
                            &issue_id,
                            issue_assignees.clone(),
                            issue_labels.clone(),
                            &close_reason,
                            &close_pull_request.clone(),
                            &closer_login.clone(),
                        )
                        .await;

                        all_issues.push(IssueClosed {
                            issue_id: issue_id,
                            issue_labels: issue_labels,
                            issue_assignees: Some(issue_assignees),
                            issue_linked_pr: Some(close_pull_request),
                            linked_pr_title: Some(close_pr_title),
                            close_reason: Some(close_reason),
                            closer_login: Some(closer_login),
                            issue_status: Some(potential_problems_summary),
                        });
                    }
                }

                if let Some(pageInfo) = search.pageInfo {
                    if pageInfo.hasNextPage {
                        after_cursor = pageInfo.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }
    Ok(all_issues)
}

pub async fn issue_checker(
    issue_id: &str,
    issue_assignees: Vec<String>,
    issue_labels: Vec<String>,
    close_reason: &str,
    close_pull_request: &str,
    close_author: &str,
) -> String {
    let  potential_problems_summary = String::new();
    let negative_labels = vec!["spam", "invalid"];
    if issue_labels
        .iter()
        .any(|label| negative_labels.contains(&label.as_str()))
    {
        // Do something
    }

    if close_author == "bot" {
        // Do something
    }

    if close_reason == "some_strange" {
        // Do something
    }
    if !issue_assignees.contains(&"intended_id".to_string()) {
        // Do something
    }

    potential_problems_summary
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct OuterPull {
    pub pull_id: String, // url of pull_request
    pub pull_title: String,
    pub pull_author: Option<String>,
    pub project_id: String,
    pub merged_by: Option<String>, // This field can be empty if the PR is not merged
    pub connected_issues: Vec<String>, // JSON data format
}

pub async fn pull_checker(
    pull_id: &str,
    pull_labels: Vec<String>,
    reviews: Vec<String>,      // authors whose review state is approved
    merged_by: Option<String>, // This field can be empty if the PR is not merged
) -> String {
    let mut potential_problems_summary = String::new();
    let negative_labels = vec!["spam", "invalid"];
    if pull_labels
        .iter()
        .any(|label| negative_labels.contains(&label.as_str()))
    {
        // Do something
    }

    if reviews.contains(&"some_bad".to_string()) {
        // Do something
    }

    if merged_by == Some("bot".to_string()) {
        // Do something
    }

    potential_problems_summary
}

#[allow(non_snake_case)]
pub async fn search_pull_requests(query: &str) -> anyhow::Result<Vec<OuterPull>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        issueCount: Option<i32>,
        nodes: Option<Vec<PullRequest>>,
        pageInfo: Option<PageInfo>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PullRequest {
        title: Option<String>,
        url: Option<String>,
        author: Option<Author>,
        timelineItems: Option<TimelineItems>,
        labels: Option<Labels>,
        reviews: Option<Reviews>,
        mergedBy: Option<Author>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct TimelineItems {
        nodes: Option<Vec<TimelineEvent>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct TimelineEvent {
        subject: Option<Subject>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Subject {
        url: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Labels {
        nodes: Option<Vec<Label>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Label {
        name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Reviews {
        nodes: Option<Vec<Review>>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Review {
        author: Option<Author>,
        state: Option<String>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PageInfo {
        endCursor: Option<String>,
        hasNextPage: bool,
    }

    let mut all_pulls = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _n in 0..10 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                        ... on PullRequest {{
                            title
                            url
                            author {{
                                login
                            }}
                            timelineItems(first: 5, itemTypes: [CONNECTED_EVENT]) {{
                                nodes {{
                                    ... on ConnectedEvent {{
                                        subject {{
                                            ... on Issue {{
                                                url
                                            }}
                                        }}
                                    }}
                                }}
                            }}
                            labels(first: 10) {{
                                nodes {{
                                    name
                                }}
                            }}
                            reviews(first: 5, states: [APPROVED]) {{
                                nodes {{
                                    author {{
                                        login
                                    }}
                                    state
                                }}
                            }}
                            mergedBy {{
                                login
                            }}
                        }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
            "#,
            query,
            after_cursor
                .as_ref()
                .map_or(String::from("null"), |c| format!("\"{}\"", c))
        );

        let response_body = github_http_post_gql(&query_str).await?;
        let response: GraphQLResponse = serde_json::from_slice(&response_body)?;

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for node in nodes {
                        let connected_issues = if let Some(items) = node.timelineItems {
                            if let Some(nodes) = items.nodes {
                                nodes
                                    .iter()
                                    .filter_map(|event| {
                                        event.subject.as_ref().map(|subject| subject.url.clone())
                                    })
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .flatten()
                                    .collect::<Vec<String>>()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                        let _labels = if let Some(items) = node.labels {
                            if let Some(nodes) = items.nodes {
                                nodes
                                    .iter()
                                    .map(|label| label.name.clone().unwrap_or_default())
                                    .collect::<Vec<String>>()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                        let _reviews = if let Some(items) = node.reviews {
                            if let Some(nodes) = items.nodes {
                                nodes
                                    .iter()
                                    .filter(|review| review.state.as_deref() == Some("APPROVED"))
                                    .filter_map(|review| {
                                        review
                                            .author
                                            .as_ref()
                                            .and_then(|author| author.login.clone())
                                    })
                                    .collect::<Vec<String>>()
                            } else {
                                Vec::new()
                            }
                        } else {
                            Vec::new()
                        };

                        let pull_id = node.url.clone().unwrap_or_default();
                        let project_id = pull_id
                            .clone()
                            .rsplitn(3, '/')
                            .nth(2)
                            .unwrap_or("unknown")
                            .to_string();
                        let pull_title = node.title.clone().unwrap_or_default();
                        let pull_author =
                            node.author.as_ref().and_then(|author| author.login.clone());
                        let merged_by = node
                            .mergedBy
                            .as_ref()
                            .and_then(|author| author.login.clone());

                        // let potential_problems_summary = pull_checker(
                        //     &pull_id,
                        //     labels.clone(),
                        //     reviews.clone(),
                        //     merged_by.clone(),
                        // )
                        // .await;

                        all_pulls.push(OuterPull {
                            pull_id,
                            pull_title,
                            pull_author,
                            project_id,
                            merged_by,
                            connected_issues,
                        });
                    }

                    if let Some(pageInfo) = search.pageInfo {
                        if pageInfo.hasNextPage {
                            after_cursor = pageInfo.endCursor;
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    Ok(all_pulls)
}

#[allow(non_snake_case)]
pub async fn search_mock_user(query: &str) -> anyhow::Result<Vec<(String, String, String)>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        issueCount: Option<i32>,
        nodes: Option<Vec<Issue>>,
        pageInfo: Option<PageInfo>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct PageInfo {
        endCursor: Option<String>,
        hasNextPage: bool,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Issue {
        participants: Option<Participants>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Participants {
        nodes: Option<Vec<Participant>>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Participant {
        login: Option<String>,
        avatarUrl: Option<String>,
        email: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..10 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                        ... on Issue {{
                            participants(first: 10) {{
                                totalCount
                                nodes {{
                                    login
                                    avatarUrl
                                    email
                                }}
                            }}
                        }}
                    }}
                    pageInfo {{
                        endCursor
                        hasNextPage
                    }}
                }}
            }}
            "#,
            query,
            after_cursor
                .as_ref()
                .map_or(String::from("null"), |c| format!("\"{}\"", c)),
        );

        let response_body = github_http_post_gql(&query_str)
            .await
            .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

        let response: GraphQLResponse = serde_json::from_slice(&response_body)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        if let Some(participants) = issue.participants {
                            if let Some(nodes) = participants.nodes {
                                for participant in nodes {
                                    all_issues.push((
                                        participant.login.unwrap_or_default(),
                                        participant.avatarUrl.unwrap_or_default(),
                                        participant.email.unwrap_or_default(),
                                    ));
                                }
                            }
                        }
                    }
                }

                if let Some(pageInfo) = search.pageInfo {
                    if pageInfo.hasNextPage {
                        after_cursor = pageInfo.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(all_issues)
}
