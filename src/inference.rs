use crate::schema::{Extraction, response_schema};
use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

const FULL_PROMPT: &str = "Read only visible League of Legends HUD values. Return null for every unreadable or absent value. Do not infer. game_time_seconds is the in-game clock in seconds, never the video time. champion must be a displayed champion name. Return only data matching the supplied schema.";
const BOTTOM_HUD_PROMPT: &str = "This image is the bottom player HUD of a League of Legends game. Read only clearly visible player champion, level, KDA, creep score, health, and mana values. Set game_time_seconds to null. Return null for unreadable values. Do not infer. Return only data matching the supplied schema.";
const TOP_HUD_PROMPT: &str = "This image is the top HUD of a League of Legends game. Read only the clearly visible in-game clock and convert it to seconds. Set all other fields to null. Do not infer. Return only data matching the supplied schema.";

#[derive(Deserialize)]
struct Generate {
    response: String,
    #[serde(default)]
    thinking: String,
}
pub async fn check_model(client: &Client, endpoint: &str, model: &str) -> Result<()> {
    let v: serde_json::Value = client
        .get(format!("{endpoint}/api/tags"))
        .send()
        .await
        .context("contact Ollama")?
        .error_for_status()?
        .json()
        .await?;
    let exists = v["models"]
        .as_array()
        .is_some_and(|xs| xs.iter().any(|x| x["name"].as_str() == Some(model)));
    anyhow::ensure!(exists, "Ollama model {model} is not installed");
    Ok(())
}
pub async fn extract(
    client: &Client,
    endpoint: &str,
    model: &str,
    jpeg: &[u8],
    max_tokens: u16,
    prompt_profile: &str,
) -> Result<Extraction> {
    let prompt = match prompt_profile {
        "full" => FULL_PROMPT,
        "bottom-hud" => BOTTOM_HUD_PROMPT,
        "top-hud" => TOP_HUD_PROMPT,
        _ => anyhow::bail!("prompt profile must be full, bottom-hud, or top-hud"),
    };
    let body = serde_json::json!({"model":model,"prompt":prompt,"images":[STANDARD.encode(jpeg)],"format":response_schema(),"stream":false,"options":{"temperature":0,"num_predict":max_tokens}});
    let r: Generate = client
        .post(format!("{endpoint}/api/generate"))
        .timeout(Duration::from_secs(120))
        .json(&body)
        .send()
        .await
        .context("request Ollama")?
        .error_for_status()?
        .json()
        .await
        .context("decode Ollama response")?;
    anyhow::ensure!(
        !r.response.trim().is_empty(),
        "model returned an empty response; thinking: {}",
        r.thinking
    );
    let mut extraction: Extraction = serde_json::from_str(&r.response)
        .with_context(|| format!("decode model JSON: {}", r.response))?;
    crate::schema::add_uncertain_fields(&mut extraction);
    Ok(extraction)
}
