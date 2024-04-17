use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_join::*;
use gosim_project::db_manipulate::*;
use gosim_project::db_populate::*;
use gosim_project::issue_tracker::*;
use gosim_project::the_runner::*;
use mysql_async::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use webhook_flows::{
    create_endpoint, request_handler,
    route::{get, route, RouteError, Router},
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

async fn trigger(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    let pool: Pool = get_pool().await;
    // let _ = note_issues(&pool).await;

    let repos = "repo:WasmEdge/wasmedge-db-examples repo:WasmEdge/www repo:WasmEdge/docs repo:WasmEdge/llvm-windows repo:WasmEdge/wasmedge-rust-sdk repo:WasmEdge/YOLO-rs repo:WasmEdge/proxy-wasm-cpp-host repo:WasmEdge/hyper-util repo:WasmEdge/hyper repo:WasmEdge/h2 repo:WasmEdge/wasmedge_hyper_demo repo:WasmEdge/tokio-rustls repo:WasmEdge/mysql_async_wasi repo:WasmEdge/mediapipe-rs repo:WasmEdge/wasmedge_reqwest_demo repo:WasmEdge/reqwest repo:WasmEdge/.github repo:WasmEdge/mio repo:WasmEdge/elasticsearch-rs-wasi repo:WasmEdge/oss-fuzz repo:WasmEdge/wasm-log-flex repo:WasmEdge/wasmedge_sdk_async_wasi repo:WasmEdge/tokio repo:WasmEdge/rust-mysql-simple-wasi repo:WasmEdge/GSoD2023 repo:WasmEdge/llm-agent-sdk repo:WasmEdge/sqlx repo:WasmEdge/rust-postgres repo:WasmEdge/redis-rs";

    // let repo_data = get_projects_as_repo_list(&pool, 1).await;
    let query ="label:hacktoberfest label:hacktoberfest-accepted is:issue created:>2023-10-01 updated:2023-10-03T05:00:00..2023-10-03T06:00:00 -label:spam -label:invalid";
    let query = "label:hacktoberfest is:issue updated:>2024-04-16 -label:spam -label:invalid";
    for issue in search_issues_comment(query).await.expect("msg") {
        log::info!("{:?}", issue.issue_id);
        let _ = add_issues_comment(&pool, issue).await;
    }

    // let _ = pull_master(&pool).await;
    // let _ = run_hourly(&pool).await;
}
