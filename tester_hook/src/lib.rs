use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_join::*;
use gosim_project::db_manipulate::*;
use gosim_project::db_populate::*;
use gosim_project::issue_tracker::*;
use gosim_project::the_runner::*;
use gosim_project::vector_search::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use webhook_flows::{
    create_endpoint, request_handler,
    route::{get, post, route, RouteError, Router},
    send_response,
};

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
    dotenv().ok();
    logger::init();

    let mut router = Router::new();
    router.insert("/run", vec![get(trigger)]).unwrap();
    router
        .insert("/comment", vec![post(get_comments_by_post_handler)])
        .unwrap();
    router
        .insert("/vector", vec![post(check_vdb_by_post_handler)])
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

async fn get_comments_by_post_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct IssueId {
        pub issue_id: String,
    }

    let load: IssueId = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };
    let pool: Pool = get_pool().await;

    let issue_id = load.issue_id;
    match get_comments_by_issue_id(&pool, &issue_id).await {
        Ok(result) => {
            let result_str = json!(result).to_string();

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
                result_str.as_bytes().to_vec(),
            );
        }
        Err(e) => {
            log::error!("Error: {:?}", e);
        }
    }
}
async fn check_vdb_by_post_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct VectorLoad {
        pub issue_id: Option<String>,
        pub collection_name: Option<String>,
        pub text: Option<String>,
    }

    let load: VectorLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };
    let pool: Pool = get_pool().await;
    let mut out = String::new();
    if let Some(text) = load.text {
        match search_collection(&text, "gosim_search").await {
            Ok(search_result) => {
                out = json!(search_result).to_string();
            }
            Err(e) => {
                log::error!("Error: {:?}", e);
            }
        }
    }
    if let Some(collection_name) = load.collection_name {
        let result = check_vector_db(&collection_name).await;
        out = json!(result).to_string();
    }

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
        out.as_bytes().to_vec(),
    );
}
async fn trigger(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    let pool: Pool = get_pool().await;
    // let _ = note_issues(&pool).await;

    let repos = "repo:WasmEdge/wasmedge-db-examples repo:WasmEdge/www repo:WasmEdge/docs repo:WasmEdge/llvm-windows repo:WasmEdge/wasmedge-rust-sdk repo:WasmEdge/YOLO-rs repo:WasmEdge/proxy-wasm-cpp-host repo:WasmEdge/hyper-util repo:WasmEdge/hyper repo:WasmEdge/h2 repo:WasmEdge/wasmedge_hyper_demo repo:WasmEdge/tokio-rustls repo:WasmEdge/mysql_async_wasi repo:WasmEdge/mediapipe-rs repo:WasmEdge/wasmedge_reqwest_demo repo:WasmEdge/reqwest repo:WasmEdge/.github repo:WasmEdge/mio repo:WasmEdge/elasticsearch-rs-wasi repo:WasmEdge/oss-fuzz repo:WasmEdge/wasm-log-flex repo:WasmEdge/wasmedge_sdk_async_wasi repo:WasmEdge/tokio repo:WasmEdge/rust-mysql-simple-wasi repo:WasmEdge/GSoD2023 repo:WasmEdge/llm-agent-sdk repo:WasmEdge/sqlx repo:WasmEdge/rust-postgres repo:WasmEdge/redis-rs";

    // let repo_data = get_projects_as_repo_list(&pool, 1).await;
    let query ="label:hacktoberfest label:hacktoberfest-accepted is:issue created:>2023-10-01 updated:2023-10-03T05:00:00..2023-10-03T06:00:00 -label:spam -label:invalid";
    let query = "label:hacktoberfest is:issue updated:>2024-04-16 -label:spam -label:invalid";

    let _ = create_my_collection(1536, "gosim_search").await;

    let issue_id = "https://github.com/ianshulx/React-projects-for-beginners/issues/60";
    if let Ok(res) = get_comments_by_issue_id(&pool, &issue_id).await {
        println!("{:?}", res);
    }

    // for issue in get_issues_from_db().await.expect("msg") {
    //     log::info!("{:?}", issue.0);
    //     let _ = upload_to_collection(&issue.0, Some(issue.1.clone()), issue.2, None).await;
    //     let _ = add_indexed_id(&pool, &issue.0).await;
    // }
    // let _ = check_vector_db("gosim_search").await;

    // for project in get_projects_from_db().await.expect("msg") {
    //     log::info!("{:?}", project.0);
    //     let _ = upload_to_collection(&project.0, None, None, project.1).await;
    //     let _ = add_indexed_id(&pool, &project.0).await;
    // }
    // let _ = check_vector_db("gosim_search").await;

    // let _ = pull_master(&pool).await;
    // let _ = run_hourly(&pool).await;
}
