use flowsnet_platform_sdk::logger;
use openai_flows::{chat, embeddings::EmbeddingsInput, OpenAIFlows};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str;
use vector_store_flows::*;
use webhook_flows::{create_endpoint, request_handler, send_response};

pub async fn upload_to_collection(issue_id: &str, payload: &str) {
    let collection_name = env::var("collection_name").unwrap_or("gosim_vector".to_string());
    let vector_size: u64 = 1536;
    let mut id: u64 = 0;

    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(3);

    let input = EmbeddingsInput::String(line.clone());
    match openai.create_embeddings(input).await {
        Ok(r) => {
            for v in r.iter() {
                let p = Point {
                    id: PointId::Num(id),
                    vector: v.iter().map(|n| *n as f32).collect(),
                    payload: json!({"text": line}).as_object().map(|m| m.to_owned()),
                };

                if let Err(e) = upsert_points(collection_name, p).await {
                    log::error!("Cannot upsert into database! {}", e);
                    send_success("Cannot upsert into database!");
                    return;
                }

                log::debug!("Created vector {} with length {}", id, v.len());
                id += 1;
            }
        }
        Err(e) => {
            log::error!("OpenAI returned an error: {}", e);
        }
    }
}

pub async fn check_vector_db(collection_name: &str) {
    match collection_info(collection_name).await {
        Ok(ci) => {
            log::debug!(
                "There are {} vectors in collection `{}`",
                ci.points_count,
                collection_name
            );
            send_success(&format!(
                "Successfully inserted {} records. The collection now has {} records in total.",
                points_count, ci.points_count
            ));
        }
        Err(e) => {
            log::error!("Cannot get collection stat {}", e);
            send_success("Cannot upsert into database!");
        }
    }
}

pub async fn send_success(body: &str) {
    send_response(
        200,
        vec![(String::from("content-type"), String::from("text/html"))],
        body.as_bytes().to_vec(),
    );
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
    collection_name: &str
) -> anyhow::Result<Vec<(u64, String)>> {
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(3);

    let question_vector = match
        openai.create_embeddings(EmbeddingsInput::String(question.to_string())).await
    {
        Ok(r) => {
            if r.len() < 1 {
                log::error!("LLM returned no embedding for the question");
                return Err(anyhow::anyhow!("LLM returned no embedding for the question"));
            }
            r[0]
                .iter()
                .map(|n| *n as f32)
                .collect()
        }
        Err(_e) => {
            log::error!("LLM returned an error: {}", _e);
            return Err(anyhow::anyhow!("LLM returned no embedding for the question"));
        }
    };

    let p = PointsSearchParams {
        vector: question_vector,
        limit: 5,
    };

    match search_points(&collection_name, &p).await {
        Ok(sp) => {
            for p in sp.iter() {
                log::debug!(
                    "Received vector score={} and text={}",
                    p.score,
                    first_x_chars(
                        p.payload.as_ref().unwrap().get("text").unwrap().as_str().unwrap(),
                        256
                    )
                );
                let p_text = p.payload.as_ref().unwrap().get("text").unwrap().as_str().unwrap();
                let p_id = match p.id {
                    PointId::Num(i) => i,
                    _ => 0,
                };
                if p.score > 0.75 {
                    rag_content.push((p_id, p_text.to_string()));
                }
            }
        }
        Err(e) => {
            log::error!("Vector search returns error: {}", e);
        }
    }
    Ok(rag_content)
}