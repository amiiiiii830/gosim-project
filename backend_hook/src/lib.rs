use flowsnet_platform_sdk::logger;
use gosim_project::db_manipulate::*;
use gosim_project::db_populate::*;
use gosim_project::vector_search::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use webhook_flows::{
    create_endpoint, request_handler,
    route::{get, post, route, RouteError, Router},
    send_response,
};

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BodyLoad {
    pub issue_id: Option<String>,
    pub issue_budget: Option<i64>,
    pub admin_feedback: Option<String>,
    pub issue_budget_approved: Option<bool>,
    pub review_status_flipper: Option<bool>,
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler(get, post)]
async fn handler(
    _headers: Vec<(String, String)>,
    _subpath: String,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    logger::init();

    let mut router = Router::new();
    router
        .insert(
            "/issues",
            vec![
                get(list_issues_handler),
                post(list_issues_by_status_handler),
            ],
        )
        .unwrap();
    router
        .insert("/issue", vec![post(get_issue_by_post_handler)])
        .unwrap();
    router
        .insert("/projects", vec![get(list_projects_handler)])
        .unwrap();
    router
        .insert("/budget", vec![post(approve_issue_budget_handler)])
        .unwrap();
    router
        .insert("/search", vec![post(search_handler)])
        .unwrap();
    router
        .insert("/decline", vec![post(batch_decline_issue_handler)])
        .unwrap();

    router
        .insert("/conclude", vec![post(conclude_issue_handler)])
        .unwrap();

    if let Err(e) = route(router).await {
        match e {
            RouteError::NotFound => {
                send_response(404, vec![], b"No route matched".to_vec());
            }
            RouteError::MethodNotAllowed => {
                send_response(405, vec![], b"Method not allowed".to_vec());
            }
        }
    }
}

async fn approve_issue_budget_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    let load: BodyLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };

    let issue_budget = load.issue_budget.unwrap_or_default();
    let issue_id = load.issue_id.unwrap_or_default();
    let pool = get_pool().await;
    let _ = assign_issue_budget_in_db(&pool, &issue_id, issue_budget).await;
}

async fn search_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct SearchLoad {
        pub query: String,
    }

    // #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    // pub struct SearchResult {
    //     pub issue_or_project_id: String,
    //     pub text: String,
    // }

    let load: SearchLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };

    let query = load.query;
    match search_collection(&query, "gosim_search").await {
        Ok(search_result) => {
            let search_result_obj = search_result[0].clone();
            // let search_result_obj: SearchResult = serde_json::from_slice(&search_result).unwrap();
            let search_result_str = json!(search_result_obj.0).to_string();

            send_response(
                200,
                vec![
                    (
                        String::from("content-type"),
                        String::from("application/json"),
                    ),
                    (
                        String::from("Access-Control-Allow-Origin"),
                        String::from("*"),
                    ),
                ],
                search_result_str.as_bytes().to_vec(),
            );
        }
        Err(e) => {
            log::error!("Error: {:?}", e);
        }
    }
}

async fn conclude_issue_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    let load: BodyLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };

    let approve = load.issue_budget_approved.unwrap_or_default();
    let issue_id = load.issue_id.unwrap_or_default();
    let pool = get_pool().await;
    if approve {
        let _ = conclude_issue_in_db(&pool, &issue_id).await;
    }
}

async fn batch_decline_issue_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize)]
    struct IssueIds {
        issue_ids: Vec<String>,
    }
    let load: IssueIds = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse IssueSubset: {}", _e);
            return;
        }
    };

    let issue_ids = load.issue_ids;
    let pool = get_pool().await;
    match batch_decline_issues_in_db(&pool, issue_ids).await {
        Ok(_) => {
            send_response(
                200,
                vec![
                    (String::from("content-type"), String::from("plain/text")),
                    (
                        String::from("Access-Control-Allow-Origin"),
                        String::from("*"),
                    ),
                ],
                "all issue_ids successfully processed".as_bytes().to_vec(),
            );
        }
        Err(failed_ids) => {
            log::error!("Error, failed processing these: {:?}", failed_ids);

            send_response(
                500,
                vec![
                    (
                        String::from("content-type"),
                        String::from("application/json"),
                    ),
                    (
                        String::from("Access-Control-Allow-Origin"),
                        String::from("*"),
                    ),
                ],
                format!("{:?}", failed_ids).as_bytes().to_vec(),
            );
        }
    }
}

async fn list_issues_by_status_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    let page = match _qry
        .get("page")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page' parameter");
            return;
        }
    };

    let page_size = match _qry
        .get("page_size")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page_size' parameter");
            return;
        }
    };
    log::error!("page: {}, page_size: {}", page, page_size);

    #[derive(Serialize, Deserialize)]
    struct ReviewStatus {
        review_status: String,
    }
    let load: ReviewStatus = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse IssueSubset: {}", _e);
            return;
        }
    };

    let review_status = load.review_status;
    let pool = get_pool().await;
    match list_issues_by_status(&pool, &review_status, page, page_size).await {
        Ok(issues_obj) => {
            let issues_str = json!(issues_obj).to_string();

            send_response(
                200,
                vec![
                    (
                        String::from("content-type"),
                        String::from("application/json"),
                    ),
                    (
                        String::from("Access-Control-Allow-Origin"),
                        String::from("*"),
                    ),
                ],
                issues_str.as_bytes().to_vec(),
            );
        }
        Err(e) => {
            log::error!("Error: {:?}", e);
        }
    }
}

async fn get_issue_by_post_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize)]
    struct IssueId {
        issue_id: String,
    }
    let load: IssueId = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse IssueSubset: {}", _e);
            return;
        }
    };

    let issue_id = &load.issue_id;

    log::info!("Issue_id: {}", issue_id);
    let pool = get_pool().await;

    let issue = get_issue_w_comments_by_id(&pool, issue_id)
        .await
        .expect("msg");

    let issues_str = json!(issue).to_string();
    log::info!("issues_str: {}", issues_str);

    send_response(
        200,
        vec![
            (
                String::from("content-type"),
                String::from("application/json"),
            ),
            (
                String::from("Access-Control-Allow-Origin"),
                String::from("*"),
            ),
        ],
        issues_str.as_bytes().to_vec(),
    );
}

async fn list_issues_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    log::info!("Received query parameters: {:?}", _qry);

    let page = match _qry
        .get("page")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page' parameter");
            return;
        }
    };

    let page_size = match _qry
        .get("page_size")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page_size' parameter");
            return;
        }
    };
    log::error!("page: {}, page_size: {}", page, page_size);
    let pool = get_pool().await;

    let issues_obj = list_issues(&pool, page, page_size).await.expect("msg");

    let issues_str = format!("{:?}", issues_obj);
    // log::error!("issues_str: {}", issues_str);

    send_response(
        200,
        vec![
            (String::from("content-type"), String::from("text/html")),
            (
                String::from("Access-Control-Allow-Origin"),
                String::from("*"),
            ),
        ],
        issues_str.as_bytes().to_vec(),
    );
}

async fn list_projects_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    log::info!("Received query parameters: {:?}", _qry);

    let page = match _qry
        .get("page")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page' parameter");
            return;
        }
    };

    let page_size = match _qry
        .get("page_size")
        .and_then(|v| v.as_str().and_then(|s| s.parse::<usize>().ok()))
    {
        Some(m) if m > 0 => m,
        _ => {
            log::error!("Invalid or missing 'page_size' parameter");
            return;
        }
    };
    log::error!("page: {}, page_size: {}", page, page_size);
    let pool = get_pool().await;

    let projects_obj = list_projects(&pool, page, page_size).await.expect("msg");

    let projects_str = json!(projects_obj).to_string();
    log::error!("projects_str: {}", projects_str);

    send_response(
        200,
        vec![(String::from("content-type"), String::from("text/html"))],
        projects_str.as_bytes().to_vec(),
    );
}
