use openai_flows::{
    chat::{ChatModel, ChatOptions},
    OpenAIFlows,
};
use regex::Regex;

pub async fn chain_of_chat(
    sys_prompt_1: &str,
    usr_prompt_1: &str,
    _chat_id: &str,
    gen_len_1: u16,
    usr_prompt_2: &str,
    gen_len_2: u16,
) -> anyhow::Result<String> {
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(2);

    let co_1 = ChatOptions {
        model: ChatModel::GPT35Turbo,
        system_prompt: Some(sys_prompt_1),
        max_tokens: Some(gen_len_1),
        ..Default::default()
    };

    let co_2 = ChatOptions {
        model: ChatModel::GPT35Turbo,
        system_prompt: Some(usr_prompt_2),
        max_tokens: Some(gen_len_2),
        ..Default::default()
    };

    match openai
        .chat_completion("summarizer", usr_prompt_1, &co_1)
        .await
    {
        Ok(res) => {
            log::info!("step 1 chat: {:?}", res);

            match openai
                .chat_completion("summarizer", usr_prompt_2, &co_2)
                .await
            {
                Ok(r) => {
                    log::info!("step 2 chat: {:?}", r);
                    return Ok(r.choice);
                }
                Err(_e) => {
                    return Err(anyhow::anyhow!("openai generation error, step 2: {_e}"));
                }
            }
        }
        Err(_e) => {
            return Err(anyhow::anyhow!("openai generation error, step 1: {_e}"));
        }
    }
}

pub async fn chat_inner_async(
    system_prompt: &str,
    user_input: &str,
    max_token: u16,
) -> anyhow::Result<String> {
    let mut openai = OpenAIFlows::new();
    openai.set_retry_times(2);

    let co = ChatOptions {
        model: ChatModel::GPT35Turbo,
        restart: true,
        system_prompt: Some(system_prompt),
        max_tokens: Some(max_token),
        ..Default::default()
    };

    match openai.chat_completion("summarizer", user_input, &co).await {
        Ok(r) => {
            log::info!("one step summarizer: {:?}", r.choice.clone());
            return Ok(r.choice);
        }
        Err(_e) => {
            return Err(anyhow::anyhow!("openai generation error, inner: {_e}"));
        }
    }
}

pub fn parse_summary_and_keywords(input: &str) -> (String, Vec<String>) {
    let summary_regex = Regex::new(r#""summary":\s*"([^"]*)""#).unwrap();
    let keywords_regex = Regex::new(r#""keywords":\s*\[?([^}\]]*)\]?"#).unwrap();

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
                .map(|s| s.trim().trim_matches(|c: char| c == '"' || c == '}' || c == '\n').to_string())
                .filter(|s| !s.is_empty()) // Filter out empty strings after splitting.
                .collect()
        });

    (summary, keywords)
}

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
