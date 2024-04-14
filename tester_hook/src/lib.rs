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
    let _ = note_issues(&pool).await;

    // let _ = pull_master(&pool).await;
    // let _ = run_hourly(&pool).await;
}

