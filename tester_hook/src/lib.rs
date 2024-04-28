use dotenv::dotenv;
use flowsnet_platform_sdk::logger;
use gosim_project::db_join::*;
use gosim_project::db_manipulate::*;
use gosim_project::db_populate::*;
use gosim_project::issue_paced_tracker::*;
use gosim_project::llm_utils::chat_inner_async;
use gosim_project::the_paced_runner::*;
use gosim_project::vector_search::*;
use mysql_async::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use vector_store_flows::delete_collection;
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
    router.insert("/run", vec![post(trigger)]).unwrap();
    // router
    //     .insert("/deep", vec![post(check_deep_handler)])
    //     .unwrap();
    router
        .insert("/comment", vec![post(get_comments_by_post_handler)])
        .unwrap();
    router
        .insert("/vector", vec![post(check_vdb_by_post_handler)])
        .unwrap();
    router
        .insert("/vector/create", vec![post(create_vdb_handler)])
        .unwrap();
    router
        .insert("/vector/delete", vec![post(delete_vdb_handler)])
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
    if let Some(text) = load.text {
        match search_collection(&text, "gosim_search").await {
            Ok(search_result) => {
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
                    json!(search_result).to_string().as_bytes().to_vec(),
                );
            }
            Err(e) => {
                log::error!("Error: {:?}", e);
            }
        }
    }
    if let Some(collection_name) = load.collection_name {
        let result = check_vector_db(&collection_name).await;
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
            json!(result).to_string().as_bytes().to_vec(),
        );
    }
}
async fn _check_deep_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct VectorLoad {
        pub text: Option<String>,
    }

    if let Ok(load) = serde_json::from_slice::<VectorLoad>(&_body) {
        if let Some(text) = load.text {
            log::info!("text: {text}");
            if let Ok(reply) = chat_inner_async("you're an AI assistant", &text, 100).await {
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
                    json!(reply).to_string().as_bytes().to_vec(),
                );
            }
        }
    }
}

async fn delete_vdb_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct VectorLoad {
        pub collection_name: Option<String>,
    }

    let load: VectorLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };
    if let Some(collection_name) = load.collection_name {
        if let Err(e) = delete_collection(&collection_name).await {
            log::error!("Error deleting vector db: {:?}", e);
        }

        let result = check_vector_db(&collection_name).await;
        let out = json!(result).to_string();

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
}
async fn create_vdb_handler(
    _headers: Vec<(String, String)>,
    _qry: HashMap<String, Value>,
    _body: Vec<u8>,
) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct VectorLoad {
        pub collection_name: Option<String>,
    }

    let load: VectorLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };
    if let Some(collection_name) = load.collection_name {
        if let Err(e) = create_my_collection(1536, &collection_name).await {
            log::error!("Error creating vector db: {:?}", e);
        }

        let result = check_vector_db(&collection_name).await;
        let out = json!(result).to_string();

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
}
async fn trigger(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    #[derive(Serialize, Deserialize, Clone, Debug, Default)]
    pub struct FuncLoad {
        pub func_ids: Vec<String>,
    }

    let load: FuncLoad = match serde_json::from_slice(&_body) {
        Ok(obj) => obj,
        Err(_e) => {
            log::error!("failed to parse body: {}", _e);
            return;
        }
    };
    let pool: Pool = get_pool().await;
    log::info!("func_id to run: {:?}", load.func_ids);

    for func_id in load.func_ids {
        let _ = match func_id.as_str() {
            "1" => popuate_dbs_save_issues_open(&pool).await,
            "2" => open_master(&pool).await,
            "3" => popuate_dbs_save_issues_assigned(&pool).await,
            "4" => assigned_master(&pool).await,
            "5" => popuate_dbs_save_issues_closed(&pool).await,
            "6" => closed_master(&pool).await,
            "7" => popuate_dbs_fill_projects(&pool).await,
            "8" => master_project(&pool).await,
            "9" => popuate_dbs_save_pull_requests(&pool).await,
            "10" => project_master_back_sync(&&pool).await,
            "11" => populate_vector_db(&pool).await,
            "12" => popuate_dbs_save_issues_comment(&pool).await,
            "13" => sum_budget_to_project(&pool).await,
            "14" => remove_pull_by_issued_linked_pr(&pool).await,
            "15" => delete_issues_open_assigned_closed(&pool).await,
            // "16" => force_issue_to_summary_update_db(&pool).await,
            _ => panic!(),
        };
    }
}

pub async fn run_hourly(pool: &Pool) -> anyhow::Result<()> {
    // let _ = popuate_dbs(pool).await?;
    // let _ = join_ops(pool).await?;
    // let _ = cleanup_ops(pool).await?;
    let _ = populate_vector_db(pool).await;
    Ok(())
}
pub async fn popuate_dbs(pool: &Pool) -> anyhow::Result<()> {
    let query_open =
        "label:hacktoberfest label:hacktoberfest-accepted is:issue closed:2023-10-18..2023-10-20 -label:spam -label:invalid";

    let open_issue_obj: Vec<IssueOpen> = search_issues_open(&query_open).await?;
    let len = open_issue_obj.len();
    log::info!("Open Issues recorded: {:?}", len);
    for issue in open_issue_obj {
        let _ = add_issues_open(pool, &issue).await;

        let _ = summarize_issue_add_in_db(pool, &issue).await;
    }

    // let query_comment =
    //     "label:hacktoberfest label:hacktoberfest-accepted is:issue closed:2023-10-12..2023-10-18 -label:spam -label:invalid";
    // log::info!("query_open: {:?}", query_open);

    // let issue_comment_obj: Vec<IssueComment> = search_issues_comment(&query_comment).await?;
    // let len = issue_comment_obj.len();
    // log::info!("Issues comment recorded: {:?}", len);
    // for issue in issue_comment_obj {
    //     let _ = add_issues_comment(pool, issue).await;
    // }

    // let _query_assigned =
    //     "label:hacktoberfest label:hacktoberfest-accepted is:issue closed:2023-10-12..2023-10-18 -label:spam -label:invalid";
    // let issues_assigned_obj: Vec<IssueAssigned> = search_issues_assigned(&_query_assigned).await?;
    // let len = issues_assigned_obj.len();
    // log::info!("Assigned issues recorded: {:?}", len);
    // for issue in issues_assigned_obj {
    //     let _ = add_issues_assigned(pool, issue).await;
    // }

    let query_closed =
        "label:hacktoberfest label:hacktoberfest-accepted is:issue closed:2023-10-18..2023-10-20 -label:spam -label:invalid";
    let close_issue_obj = search_issues_closed(&query_closed).await?;
    let len = close_issue_obj.len();
    log::info!("Closed issues recorded: {:?}", len);
    for issue in close_issue_obj {
        let _ = add_issues_closed(pool, issue).await;
    }

    Ok(())
}

pub async fn populate_vector_db(pool: &Pool) -> anyhow::Result<()> {
    for item in get_issues_repos_from_db().await.expect("msg") {
        log::info!("uploading to vector_db: {:?}", item.0);
        let _ = upload_to_collection(&item.0, item.1.clone()).await;
        let _ = mark_id_indexed(&pool, &item.0).await;
    }
    let _ = check_vector_db("gosim_search").await;

    Ok(())
}

pub async fn join_ops(pool: &Pool) -> anyhow::Result<()> {
    let _ = open_master(&pool).await?;
    // let _ = assigned_master(&pool).await?;

    let _ = closed_master(&pool).await?;

    let _ = master_project(&pool).await?;
    // let _ = sum_budget_to_project(&pool).await?;

    let query_repos: String = get_projects_as_repo_list(pool, 1).await?;
    log::info!("repos list: {:?}", query_repos.clone());

    let repo_data_vec: Vec<RepoData> = search_repos_in_batch(&query_repos).await?;

    for repo_data in repo_data_vec {
        log::info!("repo : {:?}", repo_data.project_id.clone());

        let _ = fill_project_w_repo_data(&pool, repo_data.clone()).await?;
        let _ = summarize_project_add_in_db(&pool, repo_data).await?;
    }

    Ok(())
}

pub async fn cleanup_ops(pool: &Pool) -> anyhow::Result<()> {
    let _ = remove_pull_by_issued_linked_pr(&pool).await?;
    let _ = delete_issues_open_assigned_closed(&pool).await?;

    Ok(())
}
