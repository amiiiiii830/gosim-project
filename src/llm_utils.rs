use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use secrecy::Secret;
use std::collections::HashMap;
use std::env;

use async_openai::{
    config::Config,
    types::{
        // ChatCompletionFunctionsArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs,
        // ChatCompletionTool, ChatCompletionToolArgs, ChatCompletionToolType,
        CreateChatCompletionRequestArgs,
    },
    Client as OpenAIClient,
};

pub async fn chain_of_chat(
    sys_prompt_1: &str,
    usr_prompt_1: &str,
    _chat_id: &str,
    gen_len_1: u16,
    usr_prompt_2: &str,
    gen_len_2: u16,
    error_tag: &str,
) -> anyhow::Result<String> {
    let mut headers = HeaderMap::new();
    let api_key = std::env::var("DEEP_API_KEY").expect("DEEP_API_KEY must be set");
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("MyClient/1.0.0"));
    let config = LocalServiceProviderConfig {
        // api_base: String::from("http://52.37.228.1:8080/v1"),
        api_base: String::from("https://api.deepinfra.com/v1/openai/chat/completions"),
        headers: headers,
        api_key: Secret::new(api_key),
        query: HashMap::new(),
    };

    let model = "meta-llama/Meta-Llama-3-8B-Instruct";

    let client = OpenAIClient::with_config(config);

    let mut messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(sys_prompt_1)
            .build()
            .expect("Failed to build system message")
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(usr_prompt_1)
            .build()?
            .into(),
    ];
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(gen_len_1)
        .model(model)
        .messages(messages.clone())
        .build()?;

    // dbg!("{:?}", request.clone());

    let chat = client.chat().create(request).await?;

    match chat.choices[0].message.clone().content {
        Some(res) => {
            log::info!("step 1 chat: {:?}", res);
        }
        None => {
            return Err(anyhow::anyhow!(error_tag.to_string()));
        }
    }

    messages.push(
        ChatCompletionRequestUserMessageArgs::default()
            .content(usr_prompt_2)
            .build()?
            .into(),
    );

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(gen_len_2)
        .model(model)
        .messages(messages)
        .build()?;

    let chat = client.chat().create(request).await?;

    match chat.choices[0].message.clone().content {
        Some(res) => {
            log::info!("step 2 chat: {:?}", res);
            Ok(res)
        }
        None => {
            return Err(anyhow::anyhow!(error_tag.to_string()));
        }
    }
}

#[derive(Clone, Debug)]
pub struct LocalServiceProviderConfig {
    pub api_base: String,
    pub headers: HeaderMap,
    pub api_key: Secret<String>,
    pub query: HashMap<String, String>,
}

impl Config for LocalServiceProviderConfig {
    fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.api_base, path)
    }

    fn query(&self) -> Vec<(&str, &str)> {
        self.query
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect()
    }

    fn api_base(&self) -> &str {
        &self.api_base
    }

    fn api_key(&self) -> &Secret<String> {
        &self.api_key
    }
}

pub async fn chat_inner_async(
    system_prompt: &str,
    user_input: &str,
    max_token: u16,
    model: &str,
) -> anyhow::Result<String> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("MyClient/1.0.0"));
    let api_key = env::var("DEEP_API_KEY").unwrap_or("deep_api_key_not_found".to_string());
    let config = LocalServiceProviderConfig {
        // api_base: String::from("http://52.37.228.1:8080/v1"),
        api_base: String::from("https://api.deepinfra.com/v1/openai/chat/completions"),
        headers: headers,
        api_key: Secret::new(api_key),
        query: HashMap::new(),
    };

    let client = OpenAIClient::with_config(config);
    let messages = vec![
        ChatCompletionRequestSystemMessageArgs::default()
            .content(system_prompt)
            .build()
            .expect("Failed to build system message")
            .into(),
        ChatCompletionRequestUserMessageArgs::default()
            .content(user_input)
            .build()?
            .into(),
    ];
    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(max_token)
        .model(model)
        .messages(messages)
        .build()?;

    let chat = match client.chat().create(request).await {
        Ok(chat) => chat,
        Err(_e) => {
            log::info!("Error getting response from OpenAI: {:?}", _e);
            return Err(anyhow::anyhow!("Failed to get reply from OpenAI: {:?}", _e));
        }
    };

    match chat.choices[0].message.clone().content {
        Some(res) => {
            // log::info!("{:?}", chat.choices[0].message.clone());
            Ok(res)
        }
        None => Err(anyhow::anyhow!("Failed to get reply from OpenAI")),
    }
}

pub fn parse_summary_and_keywords(input: &str) -> (String, Vec<String>) {
    let summary_regex = Regex::new(r#""summary":\s*"([^"]*)""#).unwrap();
    let keywords_regex = Regex::new(r#""keywords":\s*\[([^\]]*)\]"#).unwrap();

    let summary = summary_regex
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or(String::new(), |m| m.as_str().to_string());

    let keywords = keywords_regex
        .captures(input)
        .and_then(|cap| cap.get(1))
        .map_or(Vec::new(), |m| {
            m.as_str()
                .split(',')
                .map(|s| s.trim().trim_matches('"').to_string())
                .collect()
        });

    (summary, keywords)
}

/* pub fn parse_summary_and_keywords(input: &str) -> (String, Vec<String>) {
    let summary_key = r#"summary"#;
    let keywords_key = r#"keywords"#;
    let end_pattern = r#"","#;
    let mut summary = String::new();
    let mut keywords = Vec::new();

    // Extract summary
    if let Some(start) = input.find(summary_key) {
        let value_start = start + summary_key.len();
        if let Some(end) = input[value_start..].find(end_pattern) {
            summary = input[value_start..value_start + end]
                .trim()
                .trim_matches('"')
                .to_string();
        } else {
            summary = input[value_start..]
                .trim()
                .trim_matches(|c: char| c == '"' || c == '}')
                .to_string();
        }
    }

    // Extract keywords
    if let Some(start) = input.find(keywords_key) {
        let value_start = start + keywords_key.len() + 1; // Skip opening bracket [
        if let Some(end) = input[value_start..].find("]") {
            let keywords_str = &input[value_start..value_start + end];
            keywords = keywords_str
                .split(',')
                .map(|s| {
                    s.trim()
                        .trim_matches(|c: char| c == '"' || c == ' ')
                        .to_string()
                })
                .filter(|s| !s.is_empty())
                .collect();
        }
    }

    (summary, keywords)
} */

pub fn extract_summary_from_answer(input: &str) -> String {
    let trimmed_input = input.trim();
    let lines: Vec<&str> = trimmed_input.lines().collect();

    if lines.len() <= 1 {
        trimmed_input.to_string()
    } else {
        lines
            .iter()
            .skip(1)
            .skip_while(|&&line| line.trim().is_empty())
            .next() // Get the first element after skipping empty lines
            .map(|line| line.to_string()) // Convert &str to String
            .unwrap_or_else(|| "No summary located".to_string()) // Provide a default message
    }
}
