use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

pub struct EventClassification {
    pub kind: String,
    pub confidence: f64,
    pub summary: String,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct EventResponse {
    kind: String,
    confidence: f64,
    summary: String,
}

pub async fn check_model(client: &Client, endpoint: &str, model: &str) -> Result<()> {
    let tags: serde_json::Value = client
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .context("contact Ollama")?
        .error_for_status()?
        .json()
        .await?;
    let exists = tags["models"].as_array().is_some_and(|models| {
        models
            .iter()
            .any(|entry| entry["name"].as_str() == Some(model))
    });
    anyhow::ensure!(exists, "Ollama model {model} is not installed");
    Ok(())
}

pub async fn classify_event(
    client: &Client,
    endpoint: &str,
    model: &str,
    jpeg: &[u8],
) -> Result<EventClassification> {
    let prompt = "Classify only visible League of Legends broadcast evidence in this frame. Choose one: death, recall, objective, fight, or unknown. A death requires a visible death state or kill feed evidence. A recall requires a visible recall state. An objective requires a visible objective announcement, health bar, or take. A fight requires visible multi-champion combat. Return unknown when the frame does not prove one of these.";
    let format = serde_json::json!({"type":"object","properties":{
        "kind":{"type":"string","enum":["death","recall","objective","fight","unknown"]},
        "confidence":{"type":"number","minimum":0,"maximum":1},
        "summary":{"type":"string"}
    },"required":["kind","confidence","summary"],"additionalProperties":false});
    let body = serde_json::json!({"model":model,"prompt":prompt,"images":[STANDARD.encode(jpeg)],"format":format,"stream":false,"options":{"temperature":0,"num_predict":128}});
    let response: GenerateResponse = client
        .post(format!("{endpoint}/api/generate"))
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .await
        .context("request Ollama event classification")?
        .error_for_status()?
        .json()
        .await
        .context("decode Ollama event response")?;
    let event: EventResponse = serde_json::from_str(&response.response)
        .with_context(|| format!("decode event JSON: {}", response.response))?;
    Ok(EventClassification {
        kind: event.kind,
        confidence: event.confidence,
        summary: event.summary,
    })
}
