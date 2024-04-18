use flowsnet_platform_sdk::logger;
use openai_flows::{chat, embeddings::EmbeddingsInput, OpenAIFlows};
use serde_json::json;
use std::env;
use vector_store_flows::*;

pub async fn upload_to_collection(
    issue_or_project_id: &str,
    issue_assignees: Option<String>,
    issue_body: Option<String>,
    repo_readme: Option<String>,
) -> anyhow::Result<()> {
    let collection_name = env::var("collection_name").unwrap_or("gosim_search".to_string());
    let vector_size: u64 = 1536;
    // let issue_id = "https://github.com/alabulei1/a-test/issues/87";
    // let project_id = "https://github.com/alabulei1/a-test";
    let parts: Vec<&str> = issue_or_project_id.split('/').collect();
    let owner = parts[3].to_string();
    let repo = parts[4].to_string();
    let issue_number = if parts.len() > 6 {
        parts[6].parse::<i32>().unwrap_or(0)
    } else {
        0
    };

    let mut id: u64 = 0;
    let payload = match (issue_body.as_ref(), repo_readme.as_ref()) {
        (Some(body), _) => format!(
            "The issue is from the repository `{repo}` and the owner is `{owner}`, the issue_number is `{issue_number}`, it's assigned to `{issue_assignees:?}`, the body text: {body:?}"
        ),
        (_, Some(readme)) => format!(
            "The repository `{repo}` describes itself: {readme}, and the owner is `{owner}`"
        ),
        _ => return Ok(()),
    };

    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(3);

    let input = EmbeddingsInput::String(payload.clone());
    match openai.create_embeddings(input).await {
        Ok(r) => {
            for v in r.iter() {
                let p = vec![Point {
                    id: PointId::Num(id),
                    vector: v.iter().map(|n| *n as f32).collect(),
                    payload: json!({
                        "issue_or_project_id": issue_or_project_id,
                        "text": payload})
                    .as_object()
                    .map(|m| m.to_owned()),
                }];

                if let Err(e) = upsert_points(&collection_name, p).await {
                    log::error!("Cannot upsert into database! {}", e);
                    log::info!("Cannot upsert into database!");
                    return Ok(());
                }
                id += 1;
                log::debug!(
                    "Created vector {} with length {}",
                    issue_or_project_id,
                    v.len()
                );
            }
            Ok(())
        }
        Err(e) => {
            log::error!("OpenAI returned an error: {}", e);
            Err(anyhow::anyhow!("OpenAI returned an error: {}", e))
        }
    }
}

pub async fn check_vector_db(collection_name: &str) {
    match collection_info(collection_name).await {
        Ok(ci) => {
            log::info!(
                "The collection now has {} records in total.",
                ci.points_count
            );
        }
        Err(e) => {
            log::error!("Cannot get collection: {} Error: {}", collection_name, e);
        }
    }
}

pub async fn summarize_long_chunks(input: &str) -> String {
    let sys_prompt_1 = format!("You're a technical edtior bot.");
    let co = chat::ChatOptions {
        model: chat::ChatModel::GPT35Turbo16K,
        system_prompt: Some(&sys_prompt_1),
        restart: true,
        temperature: Some(0.7),
        max_tokens: Some(256),
        ..Default::default()
    };
    let usr_prompt_1 = format!(
        "To prepare for downstream question & answer task, you need to proprocess the source material, there are long chunks of text that are tool long to use as context, you need to extract the essence of such chunks, now please summarize this chunk: `{input}` into one concise paragraph, please stay truthful to the source material and handle the task in a factual manner."
    );

    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(2);

    match openai
        .chat_completion("summarize-long-chunks", &usr_prompt_1, &co)
        .await
    {
        Ok(r) => r.choice,

        Err(_e) => "".to_owned(),
    }
}

pub async fn search_collection(
    question: &str,
    collection_name: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(3);

    let question_vector = match openai
        .create_embeddings(EmbeddingsInput::String(question.to_string()))
        .await
    {
        Ok(r) => {
            if r.len() < 1 {
                log::error!("LLM returned no embedding for the question");
                return Err(anyhow::anyhow!(
                    "LLM returned no embedding for the question"
                ));
            }
            r[0].iter().map(|n| *n as f32).collect()
        }
        Err(_e) => {
            log::error!("LLM returned an error: {}", _e);
            return Err(anyhow::anyhow!(
                "LLM returned no embedding for the question"
            ));
        }
    };

    let p = PointsSearchParams {
        vector: question_vector,
        limit: 5,
    };

    let mut out = vec![];
    match search_points(&collection_name, &p).await {
        Ok(sp) => {
            for p in sp.iter() {
                let p_text = p
                    .payload
                    .as_ref()
                    .unwrap()
                    .get("text")
                    .unwrap()
                    .as_str()
                    .unwrap();

                let issue_or_project_id = p
                    .payload
                    .as_ref()
                    .unwrap()
                    .get("issue_or_project_id")
                    .unwrap()
                    .as_str()
                    .unwrap();

                log::debug!(
                    "Received vector score={} and text={}",
                    p.score,
                    p_text.chars().take(50).collect::<String>()
                );
                if p.score > 0.75 {
                    out.push((issue_or_project_id.to_string(), p_text.to_string()));
                }
            }
        }
        Err(e) => {
            log::error!("Vector search returns error: {}", e);
        }
    }
    Ok(out)
}

pub async fn create_my_collection(vector_size: u64, collection_name: &str) -> anyhow::Result<()> {
    let params = CollectionCreateParams {
        vector_size: vector_size,
    };

    if let Err(e) = create_collection(collection_name, &params).await {
        log::info!("Collection already exists");
    }

    check_vector_db(collection_name).await;
    Ok(())
}
