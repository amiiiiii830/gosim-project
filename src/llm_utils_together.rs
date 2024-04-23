use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, USER_AGENT},
    ClientBuilder,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub model: String,
}
#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub content: Option<String>,
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct ChatChoice {
    pub index: u32,
    pub message: ChatMessage,
}

#[derive(Debug, Deserialize, Serialize)]
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
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("MyClient/1.0.0"));
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&(format!("Bearer {}", api_key)))?);

    let messages = serde_json::json!([
        {"role": "system", "content": system_prompt},
        {"role": "user", "content": input}
    ]);

    let uri = "https://api.together.xyz/v1/chat/completions";
    let body = serde_json::to_vec(&serde_json::json!({
        "temperature": 0.7,
        "max_tokens": max_token,
        "model": "meta-llama/Llama-3-8b-chat-hf",
        "messages": messages,
    }))?;

    let client = ClientBuilder::new().default_headers(headers).build()?;
    let response = client.post(uri).body(body).send().await?;

    if response.status().is_success() {
        let response_body = response.text().await?;
        if let Ok(chat_response) = serde_json::from_str::<ChatResponse>(&response_body) {
            if let Some(content) = &chat_response.choices[0].message.content {
                return Ok(content.to_string());
            }
        }
        Err(anyhow::anyhow!("error deserialize ChatResposne"))

    } else {
        let error_msg = format!(
            "Failed to get a successful response: {:?}",
            response.status()
        );
        Err(anyhow::anyhow!(error_msg))
    }
}
