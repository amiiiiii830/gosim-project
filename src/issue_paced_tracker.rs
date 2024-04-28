use anyhow::anyhow;
use chrono::{DateTime, ParseError, Utc};
use http_req::{
    request::{Method, Request},
    uri::Uri,
};
use serde::{Deserialize, Serialize};
use std::env;

fn convert_datetime(merged_at: &str) -> Result<String, ParseError> {
    let datetime: DateTime<Utc> = merged_at.parse()?;
    Ok(datetime.format("%Y-%m-%d %H:%M:%S").to_string())
}

pub async fn github_http_get(url: &str, token: &str) -> anyhow::Result<Vec<u8>> {
    let mut writer = Vec::new();
    let url = Uri::try_from(url).unwrap();

    match Request::new(&url)
        .method(Method::GET)
        .header("User-Agent", "flows-network connector")
        .header("Content-Type", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .header("CONNECTION", "close")
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

pub async fn github_http_post(url: &str, body: &str) -> anyhow::Result<Vec<u8>> {
    let token = env::var("GITHUB_TOKEN").expect("github_token is required");
    let mut writer = Vec::new();

    let uri = Uri::try_from(url).expect("failed to parse url");

    let body = serde_json::json!({"body": body});

    match Request::new(&uri)
        .method(Method::POST)
        .header("User-Agent", "flows-network connector")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .header("Authorization", &format!("Bearer {}", token))
        .header("Content-Length", &body.to_string().len())
        .body(&body.to_string().into_bytes())
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

pub async fn github_http_post_gql(query: &str) -> anyhow::Result<Vec<u8>> {
    let token = env::var("GITHUB_TOKEN").expect("github_token is required");
    let base_url = Uri::try_from("https://api.github.com/graphql").unwrap();
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RepoData {
    pub project_id: String,
    pub repo_description: String,
    pub repo_readme: String,
    pub repo_stars: i64,
    pub main_language: String, // url of an issue
    pub project_logo: String,
}

pub async fn search_repos_in_batch(query: &str) -> anyhow::Result<Vec<RepoData>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        search: Option<Search>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Search {
        nodes: Option<Vec<Repo>>,
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

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Repo {
        url: String,
        description: Option<String>,
        readme: Option<Readme>,
        languages: Option<Languages>,
        stargazers: Option<Stargazers>,
        owner: Option<Owner>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Readme {
        text: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Language {
        name: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Languages {
        nodes: Option<Vec<Language>>,
    }

    let mut all_repos = Vec::new();

    let query_str = format!(
        r#"
            query {{
                search(query: "{}", type: REPOSITORY, first: 100) {{
                    repositoryCount
                    nodes {{
                        ... on Repository {{
                            url
                            description
                            stargazers {{
                                totalCount
                            }}
                            owner {{
                                avatarUrl
                            }}
                            readme: object(expression: "HEAD:README.md") {{
                                ... on Blob {{
                                    text
                                }}
                            }}
                            languages(first: 1, orderBy: {{field: SIZE, direction: DESC}}) {{
                                nodes {{
                                  name
                                }}
                            }}
                        }}
                    }}
                }}
            }}
        "#,
        query.replace("\"", "\\\""),
    );

    let response_body = github_http_post_gql(&query_str)
        .await
        .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

    let response: GraphQLResponse = serde_json::from_slice(&response_body)
        .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

    if let Some(data) = response.data {
        if let Some(search) = data.search {
            if let Some(nodes) = search.nodes {
                for repo in nodes {
                    all_repos.push(RepoData {
                        project_id: repo.url.clone(),
                        repo_description: repo.description.clone().unwrap_or_default(),
                        repo_readme: repo.readme.and_then(|r| r.text).unwrap_or_default(),
                        main_language: repo
                            .languages
                            .and_then(|l| {
                                l.nodes.and_then(|n| n.get(0).and_then(|l| l.name.clone()))
                            })
                            .unwrap_or_default(),
                        repo_stars: repo.stargazers.and_then(|s| s.totalCount).unwrap_or(0),
                        project_logo: repo.owner.and_then(|o| o.avatarUrl).unwrap_or_default(),
                    });
                }
            }
        }
    }

    Ok(all_repos)
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueAssigned {
    pub issue_id: String, // url of an issue
    pub issue_assignee: String,
    pub date_assigned: String,
}

pub async fn search_issues_assigned(query: &str) -> anyhow::Result<Vec<IssueAssigned>> {
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

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct IssueNode {
        url: Option<String>,
        timelineItems: Option<TimelineItems>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Assignee {
        login: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct TimelineItems {
        nodes: Option<Vec<AssignedEvent>>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct AssignedEvent {
        assignee: Option<Assignee>,
        createdAt: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..1 {
        let query_str = format!(
            r#"
                query {{
                    search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                        issueCount
                        nodes {{
                            ... on Issue {{
                                url
                                timelineItems(first: 1, itemTypes: [ASSIGNED_EVENT]) {{
                                    nodes {{
                                      ... on AssignedEvent {{
                                        assignee {{
                                          ... on User {{
                                            login
                                          }}
                                        }}
                                        createdAt
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
                        if let Some(timeline_items) = issue.timelineItems {
                            if let Some(nodes) = timeline_items.nodes {
                                for node in nodes {
                                    let assignee = node
                                        .assignee
                                        .as_ref()
                                        .and_then(|a| a.login.clone())
                                        .unwrap_or_default();
                                    let created_at = node.createdAt.clone().unwrap_or_default();

                                    let date_assigned =
                                        convert_datetime(&created_at).unwrap_or_default();
                                    all_issues.push(IssueAssigned {
                                        issue_id: issue.url.clone().unwrap_or_default(),
                                        issue_assignee: assignee,
                                        date_assigned,
                                    });
                                }
                            }
                        }
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueUpdated {
    pub issue_id: String, // url of an issue
    pub node_id: String,
}

pub async fn search_issues_updated(query: &str) -> anyhow::Result<Vec<IssueUpdated>> {
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

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct IssueNode {
        url: Option<String>,
        id: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..1 {
        let query_str = format!(
            r#"
                query {{
                    search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                        issueCount
                        nodes {{
                            ... on Issue {{
                                url
                                id
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
                        let issue_id = match issue.url {
                            Some(u) => u,
                            None => continue,
                        };

                        let node_id = match issue.id {
                            Some(id) => id,
                            None => continue,
                        };

                        all_issues.push(IssueUpdated { issue_id, node_id });
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueOpen {
    pub node_id: String,
    pub issue_title: String,
    pub issue_id: String,          // url of an issue
    pub issue_creator: String,     // url of an issue
    pub issue_budget: i32,         // url of an issue
    pub issue_description: String, // description of the issue, could be truncated body text
    pub project_id: String,        // url of the repo
}

pub async fn search_issues_open(query: &str) -> anyhow::Result<Vec<IssueOpen>> {
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
        id: String,
        title: String,
        url: String,
        body: Option<String>,
        author: Option<Author>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;

    for _ in 0..1 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                        ... on Issue {{
                            id
                            title
                            url
                            body
                            author {{
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

        // let test = String::from_utf8_lossy(&response_body)
        //     .chars()
        //     .take(100)
        //     .collect::<String>();
        // log::info!("search issue open response head: {test}");
        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let node_id = issue.id.clone();
                        let issue_description = issue
                            .body
                            .clone()
                            .unwrap_or_default()
                            .chars()
                            .take(8000)
                            .collect::<String>();
                        let project_id = issue
                            .url
                            .rsplitn(3, '/')
                            .nth(2)
                            .unwrap_or("wrong_project_id")
                            .to_string();
                        let issue_creator = issue
                            .author
                            .as_ref()
                            .and_then(|author| author.login.clone())
                            .unwrap_or_default();
                        let issue_budget = extract_budget(&issue_description);
                        all_issues.push(IssueOpen {
                            node_id,
                            issue_title: issue.title,
                            issue_id: issue.url, // Assuming issue.url is the issue_id
                            issue_creator,
                            issue_description,
                            issue_budget,
                            project_id,
                        });
                    }
                }

                if let Some(page_info) = search.pageInfo {
                    if page_info.hasNextPage {
                        after_cursor = page_info.endCursor;
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
pub struct IssueComment {
    pub issue_id: String,
    pub assignees: Option<Vec<String>>,
    pub comment_creator: String,
    pub comment_date: String,
    pub comment_body: String,
}

pub async fn search_issues_comment(node_ids: Vec<String>) -> anyhow::Result<Vec<IssueComment>> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        nodes: Vec<Issue>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Issue {
        url: String,
        assignees: Option<AssigneeNodes>,
        comments: Option<CommentNodes>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct AssigneeNodes {
        nodes: Vec<Assignee>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Assignee {
        name: String,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct CommentNodes {
        nodes: Vec<Comment>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Comment {
        author: Option<Author>,
        body: Option<String>,
        updatedAt: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: String,
    }

    let ids_query = node_ids.iter().map(|id| format!("\"{}\"", id)).collect::<Vec<_>>().join(", ");
    let query_str = format!(
        r#"
        query GetIssues {{
            nodes(ids: [{}]) {{
                ... on Issue {{
                    url
                    assignees(first: 5) {{
                        nodes {{
                            name
                        }}
                    }}
                    comments(first: 100, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
                        nodes {{
                            author {{
                                login
                            }}
                            body
                            updatedAt
                        }}
                    }}
                }}
            }}
        }}
        "#,
        ids_query
    );

    let response_body = github_http_post_gql(&query_str)
        .await
        .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

    let response: GraphQLResponse = serde_json::from_slice(&response_body)
        .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

    let mut all_comments = Vec::new();
    if let Some(data) = response.data {
        for issue in data.nodes {
            let assignees = issue.assignees.map(|a| a.nodes.into_iter().map(|x| x.name).collect());

            if let Some(comments) = issue.comments {
                for comment in comments.nodes {
                    let comment_creator = comment.author.map_or(String::new(), |a| a.login);
                    let comment_date = comment.updatedAt.unwrap_or_default();
                    let comment_body = comment.body.unwrap_or_default();

                    all_comments.push(IssueComment {
                        issue_id: issue.url.clone(),
                        assignees: assignees.clone(),
                        comment_creator,
                        comment_date,
                        comment_body,
                    });
                }
            }
        }
    }

    Ok(all_comments)
}



/* #[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct IssueComment {
    pub issue_id: String,        // url of an issue
    pub comment_creator: String, // url of an issue
    pub comment_date: String,
    pub comment_body: String,
}
 */
/* pub async fn search_issues_comment(query: &str) -> anyhow::Result<Vec<IssueComment>> {
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
        url: String,
        comments: Option<CommentNodes>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct CommentNodes {
        nodes: Option<Vec<Comment>>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Comment {
        author: Option<Author>,
        body: Option<String>,
        updatedAt: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
    }

    let mut all_issues = Vec::new();
    let mut after_cursor: Option<String> = None;
    // let last_hour = Utc::now() - Duration::try_hours(1).unwrap();
    let last_hour = Utc::now() - Duration::try_days(5).unwrap();

    for _ in 0..1 {
        let query_str = format!(
            r#"
            query {{
                search(query: "{}", type: ISSUE, first: 100, after: {}) {{
                    issueCount
                    nodes {{
                        ... on Issue {{
                            url
                            comments (first: 100, orderBy: {{field: UPDATED_AT, direction: DESC}}) {{
                                nodes {{
                                  author {{
                                    login
                                  }}
                                  body
                                  updatedAt
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
        // let test = String::from_utf8_lossy(&response_body)
        //     .chars()
        //     .take(100)
        //     .collect::<String>();
        // log::info!("search issue comment response head: {test}");

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let mut inner_comments_vec = Vec::new();
                        if let Some(comments) = &issue.comments {
                            if let Some(nodes) = &comments.nodes {
                                for comment in nodes {
                                    if let Some(updated_at) = &comment.updatedAt {
                                        let updated_at = DateTime::parse_from_rfc3339(updated_at)
                                            .unwrap()
                                            .with_timezone(&Utc);
                                        if updated_at > last_hour {
                                            let comment_creator = comment
                                                .author
                                                .as_ref()
                                                .and_then(|author| author.login.clone())
                                                .unwrap_or_default();
                                            inner_comments_vec.push(IssueComment {
                                                issue_id: issue.url.clone(),
                                                comment_creator,
                                                comment_date: updated_at
                                                    .format("%Y-%m-%d %H:%M:%S")
                                                    .to_string(),
                                                comment_body: comment
                                                    .body
                                                    .clone()
                                                    .unwrap_or_default(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        all_issues.extend(inner_comments_vec);
                    }
                }

                if let Some(page_info) = search.pageInfo {
                    if page_info.hasNextPage {
                        after_cursor = page_info.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(all_issues)
} */

pub fn extract_budget(body: &str) -> i32 {
    let re = regex::Regex::new(r"(?i)budget:?\s*(\d{2,3})").unwrap();
    for cap in re.captures_iter(body) {
        if let Some(match_) = cap.get(1) {
            if let Ok(b) = match_.as_str().parse::<i32>() {
                if b >= 10 && b <= 999 {
                    return b; // Returns the first match found
                }
            }
        }
    }
    0
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IssueClosed {
    pub issue_id: String, // url of an issue
    pub issue_assignees: Option<Vec<String>>,
    pub issue_linked_pr: Option<String>,
}

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

    for _ in 0..1 {
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
        // let test = String::from_utf8_lossy(&response_body)
        //     .chars()
        //     .take(100)
        //     .collect::<String>();
        // log::info!("search issue closed response head: {test}");

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for issue in nodes {
                        let _issue_labels = issue.labels.as_ref().map_or(Vec::new(), |labels| {
                            labels.nodes.as_ref().map_or(Vec::new(), |nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|label| label.name.clone())
                                    .collect()
                            })
                        });

                        let mut issue_assignees = issue.assignees.as_ref().and_then(|assignees| {
                            assignees.nodes.as_ref().map(|nodes| {
                                nodes
                                    .iter()
                                    .filter_map(|assignee| assignee.name.clone())
                                    .collect::<Vec<_>>()
                            })
                        });

                        if let Some(assignees) = &issue_assignees {
                            if assignees.is_empty() {
                                issue_assignees = None;
                            }
                        }

                        let (_close_reason, close_pull_request, _close_pr_title, _closer_login) =
                            issue.timelineItems.as_ref().map_or(
                                (None, None, None, None),
                                |items| {
                                    items
                                        .nodes
                                        .as_ref()
                                        .map_or((None, None, None, None), |nodes| {
                                            nodes
                                                .iter()
                                                .filter_map(|event| {
                                                    if let Some(closer) = &event.closer {
                                                        Some((
                                                            event.stateReason.clone(),
                                                            closer.url.clone(),
                                                            closer.title.clone(),
                                                            closer
                                                                .author
                                                                .as_ref()
                                                                .map(|author| author.login.clone()),
                                                        ))
                                                    } else {
                                                        Some((None, None, None, None))
                                                    }
                                                })
                                                .next()
                                                .unwrap_or((None, None, None, None))
                                        })
                                },
                            );

                        let issue_id = match issue.url {
                            Some(u) => u.to_string(),
                            None => continue,
                        };

                        all_issues.push(IssueClosed {
                            issue_id: issue_id,
                            issue_assignees,
                            issue_linked_pr: close_pull_request,
                        });
                    }
                }

                if let Some(page_info) = search.pageInfo {
                    if page_info.hasNextPage {
                        after_cursor = page_info.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }
    Ok(all_issues)
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct OuterPull {
    pub pull_id: String, // url of pull_request
    pub pull_title: String,
    pub pull_author: Option<String>,
    pub project_id: String,
    pub merged_at: String,
}

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
        labels: Option<Labels>,
        reviews: Option<Reviews>,
        mergedAt: Option<String>,
    }

    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Author {
        login: Option<String>,
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

    for _n in 0..1 {
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
                            mergedAt
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
        // let test = String::from_utf8_lossy(&response_body)
        //     .chars()
        //     .take(100)
        //     .collect::<String>();
        // log::info!("search pull_request response head: {test}");

        if let Some(data) = response.data {
            if let Some(search) = data.search {
                if let Some(nodes) = search.nodes {
                    for node in nodes {
                        let pull_id = node.url.clone().unwrap_or_default();
                        let project_id = pull_id
                            .clone()
                            .rsplitn(3, '/')
                            .nth(2)
                            .unwrap_or("failed_to_get_project_id")
                            .to_string();
                        let pull_title = node.title.clone().unwrap_or_default();
                        let pull_author =
                            node.author.as_ref().and_then(|author| author.login.clone());
                        let merged_at = node.mergedAt.unwrap_or_default();
                        let merged_at = convert_datetime(&merged_at).unwrap_or_default();

                        all_pulls.push(OuterPull {
                            pull_id,
                            pull_title,
                            pull_author,
                            project_id,
                            merged_at,
                        });
                    }

                    if let Some(page_info) = search.pageInfo {
                        if page_info.hasNextPage {
                            after_cursor = page_info.endCursor;
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

    for _ in 0..1 {
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

                if let Some(page_info) = search.pageInfo {
                    if page_info.hasNextPage {
                        after_cursor = page_info.endCursor;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    Ok(all_issues)
}

pub async fn get_rate_limit() -> anyhow::Result<i32> {
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct GraphQLResponse {
        data: Option<Data>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct Data {
        rateLimit: Option<RateLimit>,
    }

    #[allow(non_snake_case)]
    #[derive(Serialize, Deserialize, Clone, Default, Debug)]
    struct RateLimit {
        limit: i32,
        remaining: i32,
        used: i32,
        resetAt: String,
    }

    let query_str = r#"
        query {
            rateLimit {
                limit
                remaining
                used
                resetAt
            }
        }
    "#;

    let response_body = github_http_post_gql(&query_str)
        .await
        .map_err(|e| anyhow!("Failed to post GraphQL query: {}", e))?;

    let response: GraphQLResponse = serde_json::from_slice(&response_body)
        .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

    if let Some(data) = response.data {
        if let Some(rate_limit) = data.rateLimit {
            return Ok(rate_limit.remaining);
        }
    }

    Err(anyhow!("Failed to get rate limit"))
}

pub fn rand_token() -> String {
    use rand::seq::SliceRandom; // Import the necessary trait
    use rand::thread_rng; // Import the RNG
    let tokens = std::env::var("token_pool").unwrap_or("no token pool".to_string());

    let token_vec = tokens
        .trim()
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect::<Vec<&str>>();

    let mut rng = thread_rng();

    if let Some(token) = token_vec.choose(&mut rng) {
        token.to_string()
    } else {
        "no token available".to_string() // Handle the case where no tokens are available
    }
}
