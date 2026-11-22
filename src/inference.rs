use crate::schema::{
    Extraction, champion_response_schema, clock_response_schema, resource_response_schema,
    response_schema,
};
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

const FULL_PROMPT: &str = "Read only visible League of Legends HUD values. Return null for every unreadable or absent value. Do not infer. game_time_seconds is the in-game clock in seconds, never the video time. champion must be a displayed champion name. Return only data matching the supplied schema.";
const BOTTOM_HUD_PROMPT: &str = "This image is the bottom player HUD of a League of Legends game. Read only clearly visible player champion, level, KDA, creep score, health, and mana values. Set game_time_seconds to null. Return null for unreadable values. Do not infer. Return only data matching the supplied schema.";
const TOP_HUD_PROMPT: &str = "This image is the top HUD of a League of Legends game. Read only the clearly visible in-game clock and convert it to seconds. Set all other fields to null. Do not infer. Return only data matching the supplied schema.";
const CLOCK_PROMPT: &str = "This image contains a League of Legends in-game timer. Transcribe only the visible timer text exactly as m:ss. Do not convert it to seconds. Do not infer. Return null if any digit is unreadable.";
const RESOURCE_PROMPT: &str = "This image contains only the player health and mana HUD bars from a League of Legends game. Read the current numeric health and mana values only when clearly visible. Set game_time_seconds, champion, level, kills, deaths, assists, and cs to null. Return null for unreadable values. Do not infer. Return only data matching the supplied schema.";
const PORTRAIT_PROMPT: &str = "This image contains a League of Legends champion portrait. Identify the champion only when the portrait is clear. Set every non-champion field to null. Return null instead of guessing. Return only data matching the supplied schema.";

#[derive(Deserialize)]
struct Generate {
    response: String,
    #[serde(default)]
    thinking: String,
}

#[derive(Deserialize)]
struct ClockResponse {
    clock: Option<String>,
}

#[derive(Deserialize)]
struct ChampionResponse {
    champion: Option<String>,
}

#[derive(Deserialize)]
struct EventResponse {
    kind: String,
    confidence: f64,
    summary: String,
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
    champion_candidates: Option<&str>,
    clock_candidates: Option<&str>,
) -> Result<Extraction> {
    let (prompt, format) = match prompt_profile {
        "full" => (FULL_PROMPT.to_string(), response_schema()),
        "bottom-hud" => (BOTTOM_HUD_PROMPT.to_string(), response_schema()),
        "top-hud" => (TOP_HUD_PROMPT.to_string(), response_schema()),
        "clock-text" => {
            let prompt = match clock_candidates {
                Some(candidates) => format!(
                    "{CLOCK_PROMPT} Choose only one exact value from these candidate timer texts, or null: {candidates}."
                ),
                None => CLOCK_PROMPT.to_string(),
            };
            (prompt, clock_response_schema())
        }
        "resource-hud" => (RESOURCE_PROMPT.to_string(), resource_response_schema()),
        "portrait" => {
            let prompt = match champion_candidates {
                Some(candidates) => format!(
                    "{PORTRAIT_PROMPT} Choose only one exact name from this candidate list, or null: {candidates}."
                ),
                None => PORTRAIT_PROMPT.to_string(),
            };
            (prompt, champion_response_schema())
        }
        _ => anyhow::bail!(
            "prompt profile must be full, bottom-hud, top-hud, clock-text, resource-hud, or portrait"
        ),
    };
    let body = serde_json::json!({"model":model,"prompt":prompt,"images":[STANDARD.encode(jpeg)],"format":format,"stream":false,"options":{"temperature":0,"num_predict":max_tokens}});
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
    let mut extraction = if prompt_profile == "clock-text" {
        let response: ClockResponse = serde_json::from_str(&r.response)
            .with_context(|| format!("decode clock JSON: {}", r.response))?;
        Extraction {
            game_time_seconds: response
                .clock
                .as_deref()
                .and_then(crate::schema::parse_clock),
            raw_clock: response.clock,
            ..Default::default()
        }
    } else if prompt_profile == "portrait" {
        let response: ChampionResponse = serde_json::from_str(&r.response)
            .with_context(|| format!("decode champion JSON: {}", r.response))?;
        Extraction {
            champion: response.champion,
            ..Default::default()
        }
    } else {
        serde_json::from_str(&r.response)
            .with_context(|| format!("decode model JSON: {}", r.response))?
    };
    crate::schema::add_uncertain_fields(&mut extraction);
    Ok(extraction)
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
    let response: Generate = client
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
