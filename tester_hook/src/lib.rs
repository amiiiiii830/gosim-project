use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_join::*;
use gosim_project::db_manipulate::*;
use gosim_project::db_populate::*;
use gosim_project::issue_tracker::*;
use gosim_project::the_runner::*;
use http_req::{
    request::{Method, Request},
    uri::Uri,
};
use mysql_async::*;
use serde::{Deserialize, Serialize};
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
    logger::init();
    // let code = _qry.get("code").and_then(|m| m.as_str()).unwrap_or("");
    // let mut token = String::new();
    // match exchange_token_w_output(code).await {
    //     Ok(m) => {
    //         token = m;
    //         log::info!("Token: {:?}", token);

    //         // send_response(200, vec![], b"You've successfully registered.".to_vec());
    //     }

    //     Err(e) => {
    //         log::error!("Error: {:?}", e);
    //         // send_response(
    //         //     500,
    //         //     vec![],
    //         //     b"Something went wrong with the registration, please try again.".to_vec(),
    //         // );
    //         return;
    //     }
    // };

    // let (_, login, _, email) = get_user_profile_with_his_token(&token)
    //     .await
    //     .expect("failed to get user profile");

    // log::info!("profiled user: {:?}, {}", login, email);
    let pool: Pool = get_pool().await;

    let _ = pull_master(&pool).await;
    // let _ = run_hourly(&pool).await;
}

pub async fn github_http_post(url: &str, query: &str) -> anyhow::Result<Vec<u8>> {
    // let token = env::var("GITHUB_TOKEN").expect("github_token is required");
    let mut writer = Vec::new();

    let uri = Uri::try_from(url).expect("failed to parse url");

    match Request::new(&uri)
        .method(Method::POST)
        .header("User-Agent", "flows-network connector")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        // .header("Authorization", &format!("Bearer {}", token))
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
