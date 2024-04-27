use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    ClientBuilder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub model: String,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ChatMessage {
    pub content: Option<String>,
    pub role: Role,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    pub finish_reason: Option<String>,
    pub index: u32,
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub enum Role {
    #[serde(rename = "system")]
    System,
    #[serde(rename = "user")]
    User,
    #[serde(rename = "assistant")]
    Assistant,
    #[serde(rename = "tool")]
    Tool,
    #[serde(rename = "function")]
    Function,
}
pub async fn chat_inner_async(
    system_prompt: &str,
    input: &str,
    max_token: u16,
) -> anyhow::Result<String> {
    let mut headers = HeaderMap::new();
    let api_key = std::env::var("TOGETHER_API_KEY")?;
    // let api_key = std::env::var("AZURE_API_TOKEN")?;
    let bearer_token = format!("Bearer {}", api_key);

    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("MyClient/1.0.0"));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&bearer_token)?);
    let input_head = input.chars().take(150).collect::<String>();

    let messages = serde_json::json!([
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": input}
    ]);

    let uri = "https://api.together.xyz/v1/chat/completions";
    // let uri = "https://Meta-Llama-3-8B-Instruct-ttskb-serverless.eastus2.inference.ai.azure.com/v1/chat/completions";
    let body = serde_json::to_vec(&serde_json::json!({
        "temperature": 0.7,
        "max_tokens": max_token,
       "model": "meta-llama/Llama-3-8b-chat-hf",
        "messages": messages,
    }))?;
    //  "model": "Meta-Llama-3-8B-Instruct",

    let client = ClientBuilder::new().default_headers(headers).build()?;
    let response = client.post(uri).body(body.clone()).send().await?;

    if response.status().is_success() {
        let response_body = response.text().await?;
        if let Ok(chat_response) = serde_json::from_str::<ChatResponse>(&response_body) {
            // let finish_reason = &chat_response.choices[0]
            //     .clone()
            //     .finish_reason
            //     .unwrap_or("no finish reason found".to_string());
            // log::info!("input: {}, finish_reason: {}", input_head, finish_reason);

            if let Some(content) = &chat_response.choices[0].message.content {
                // log::info!("together summary: {}", content.to_string());
                return Ok(content.to_string());
            }
        }

    } else {
        log::info!("LLM generation failed in first round, now retrying");

        let response = client.post(uri).body(body).send().await?;

        let response_body = response.text().await?;
        let chat_response = serde_json::from_str::<ChatResponse>(&response_body)?;
        // let finish_reason = &chat_response.choices[0]
        //     .clone()
        //     .finish_reason
        //     .unwrap_or("no finish reason found".to_string());
        // log::info!("summary, second run: finish_reason: {}", finish_reason);

        return Ok(chat_response.choices[0]
            .clone()
            .message
            .content
            .unwrap_or_default());
    }
    Err(anyhow::anyhow!("error deserialize ChatResponse"))?
}
