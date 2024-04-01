use gosim_project::db_updater::*;
use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
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
        .insert("/issues", vec![get(list_issues_handler)])
        .unwrap();
    router
        .insert("/budget", vec![post(approve_issue_budget_handler)])
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
    let _ = approve_issue_budget_in_db(&pool, &issue_id, issue_budget).await;
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
    log::error!("issues_str: {}", issues_str);

    send_response(
        200,
        vec![(String::from("content-type"), String::from("text/html"))],
        issues_str.as_bytes().to_vec(),
    );
}

async fn projects_list(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    // match _qry.get("file_name") {
    //     Some(m) => match serde_json::from_value::<String>(m.clone()) {
    //         Ok(key) => "file_name".to_string(),
    //         Err(_e) => {
    //             log::error!("failed to parse file_name: {}", _e);
    //             return;
    //         }
    //     },
    //     _ => {
    //         log::error!("missing file_name");
    //         return;
    //     }
    // }
}
