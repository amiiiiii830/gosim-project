use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, USER_AGENT};
use secrecy::Secret;
use serde::Deserialize;
use std::collections::HashMap;

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
) -> anyhow::Result<String> {
    let mut headers = HeaderMap::new();
    let api_key = std::env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY must be set");
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(USER_AGENT, HeaderValue::from_static("MyClient/1.0.0"));
    let config = LocalServiceProviderConfig {
        api_base: String::from("https://api.together.xyz/v1/chat/completions"),
        headers: headers,
        api_key: Secret::new(api_key),
        query: HashMap::new(),
    };

    // stop: ['</s>', '[/INST]'],
    let model = "meta-llama/Llama-3-8b-chat-hf";
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
        Ok(chat) => {

            chat},
        Err(_e) => {
            log::info!("Error getting response from OpenAI: {:?}", _e);
            return Err(anyhow::anyhow!("Failed to get reply from OpenAI: {:?}", _e));
        }
    };
            let raw = chat.choices.to_owned();
            log::info!("Error getting response from OpenAI: {:?}", raw);

    match chat.choices[0].message.clone().content {
        Some(res) => {
            // log::info!("{:?}", chat.choices[0].message.clone());
            Ok(res)
        }
        None => Err(anyhow::anyhow!("Failed to get reply from OpenAI")),
    }
}

pub fn parse_summary_from_raw_json(input: &str) -> String {
    #[derive(Deserialize, Debug)]
    struct SummaryStruct {
        impactful: Option<String>,
        alignment: Option<String>,
        patterns: Option<String>,
        synergy: Option<String>,
        significance: Option<String>,
    }

    let summary: SummaryStruct = serde_json::from_str(input).expect("Failed to parse summary JSON");

    let fields = [
        &summary.impactful,
        &summary.alignment,
        &summary.patterns,
        &summary.synergy,
        &summary.significance,
    ];

    fields
        .iter()
        .filter_map(|&field| field.as_ref()) // Convert Option<&String> to Option<&str>
        .filter(|field| !field.is_empty()) // Filter out empty strings
        .fold(String::new(), |mut acc, field| {
            if !acc.is_empty() {
                acc.push_str(" ");
            }
            acc.push_str(field);
            acc
        })
}



pub fn parse_issue_summary_from_json(input: &str) -> anyhow::Result<Vec<(String, String)>> {
    let parsed: serde_json::Map<String, serde_json::Value> = serde_json::from_str(input)?;

    let summaries = parsed
        .iter()
        .filter_map(|(key, value)| {
            if let Some(summary_str) = value.as_str() {
                Some((key.clone(), summary_str.to_owned()))
            } else {
                None
            }
        })
        .collect::<Vec<(String, String)>>(); // Collect into a Vec of tuples

    Ok(summaries)
}


