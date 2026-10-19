// SPDX-License-Identifier: GPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Provider {
    Ollama,
    LmStudio,
}

impl Provider {
    pub fn from_config(value: &str) -> Self {
        match value {
            "lm-studio" => Self::LmStudio,
            _ => Self::Ollama,
        }
    }
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Serialize)]
struct LmStudioChatRequest<'a> {
    model: &'a str,
    system_prompt: &'static str,
    input: &'a str,
}

#[derive(Deserialize)]
struct LmStudioChatResponse {
    output: Vec<LmStudioOutput>,
}

#[derive(Deserialize)]
struct LmStudioOutput {
    content: Option<String>,
}

pub async fn generate(
    provider: Provider,
    endpoint: String,
    model: String,
    prompt: String,
) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Could not create local model client: {error}"))?;

    match provider {
        Provider::Ollama => generate_ollama(&client, endpoint, model, prompt).await,
        Provider::LmStudio => generate_lm_studio(&client, endpoint, model, prompt).await,
    }
}

async fn generate_ollama(
    client: &reqwest::Client,
    endpoint: String,
    model: String,
    prompt: String,
) -> Result<String, String> {
    let endpoint = format!("{}/api/generate", endpoint.trim_end_matches('/'));
    let response = client
        .post(&endpoint)
        .json(&GenerateRequest {
            model: &model,
            prompt: &prompt,
            stream: false,
        })
        .send()
        .await
        .map_err(|error| format!("Could not reach Ollama at {endpoint}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(format!("Ollama returned {status}: {detail}"));
    }

    response
        .json::<GenerateResponse>()
        .await
        .map(|response| response.response.trim().to_string())
        .map_err(|error| format!("Could not read Ollama's response: {error}"))
}

async fn generate_lm_studio(
    client: &reqwest::Client,
    endpoint: String,
    model: String,
    prompt: String,
) -> Result<String, String> {
    let endpoint = lm_studio_chat_endpoint(&endpoint);
    let response = client
        .post(&endpoint)
        .json(&LmStudioChatRequest {
            model: &model,
            system_prompt: "You are a careful writing assistant. Follow the user's requested transformation exactly and return only the resulting text.",
            input: &prompt,
        })
        .send()
        .await
        .map_err(|error| format!("Could not reach LM Studio at {endpoint}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let detail = response.text().await.unwrap_or_default();
        return Err(format!("LM Studio returned {status}: {detail}"));
    }

    let response = response
        .json::<LmStudioChatResponse>()
        .await
        .map_err(|error| format!("Could not read LM Studio's response: {error}"))?;
    extract_lm_studio_output(response)
}

fn extract_lm_studio_output(response: LmStudioChatResponse) -> Result<String, String> {
    let output = response
        .output
        .into_iter()
        .filter_map(|output| output.content)
        .collect::<Vec<_>>()
        .join("")
        .trim()
        .to_string();

    (!output.is_empty())
        .then_some(output)
        .ok_or_else(|| "LM Studio returned no message content.".to_string())
}

fn lm_studio_chat_endpoint(endpoint: &str) -> String {
    let endpoint = endpoint.trim_end_matches('/');

    if endpoint.ends_with("/api/v1/chat") {
        endpoint.to_string()
    } else if endpoint.ends_with("/api/v1") {
        format!("{endpoint}/chat")
    } else {
        format!("{endpoint}/api/v1/chat")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LmStudioChatResponse, LmStudioOutput, extract_lm_studio_output, lm_studio_chat_endpoint,
    };

    #[test]
    fn builds_lm_studio_native_chat_endpoint() {
        assert_eq!(
            lm_studio_chat_endpoint("http://localhost:1234"),
            "http://localhost:1234/api/v1/chat"
        );
        assert_eq!(
            lm_studio_chat_endpoint("http://localhost:1234/api/v1"),
            "http://localhost:1234/api/v1/chat"
        );
        assert_eq!(
            lm_studio_chat_endpoint("http://localhost:1234/api/v1/chat"),
            "http://localhost:1234/api/v1/chat"
        );
    }

    #[test]
    fn extracts_text_from_lm_studio_message_output() {
        let output = extract_lm_studio_output(LmStudioChatResponse {
            output: vec![LmStudioOutput {
                content: Some("Improved text".to_string()),
            }],
        });

        assert_eq!(output, Ok("Improved text".to_string()));
    }
}
