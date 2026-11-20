use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Extraction {
    pub game_time_seconds: Option<u32>,
    pub champion: Option<String>,
    pub level: Option<u8>,
    pub kills: Option<u32>,
    pub deaths: Option<u32>,
    pub assists: Option<u32>,
    pub cs: Option<u32>,
    pub health: Option<f64>,
    pub mana: Option<f64>,
    #[serde(default)]
    pub uncertain_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub video_fingerprint: String,
    pub frame_id: String,
    pub video_timestamp_ms: u64,
    pub experiment_id: String,
    pub config_fingerprint: String,
    pub model: String,
    pub extraction: Extraction,
    pub validation_errors: Vec<String>,
    pub inference_latency_ms: u128,
}

pub fn response_schema() -> serde_json::Value {
    serde_json::json!({"type":"object","properties":{
      "game_time_seconds":{"type":["integer","null"],"minimum":0},
      "champion":{"type":["string","null"]},
      "level":{"type":["integer","null"],"minimum":1,"maximum":18},
      "kills":{"type":["integer","null"],"minimum":0},
      "deaths":{"type":["integer","null"],"minimum":0},
      "assists":{"type":["integer","null"],"minimum":0},
      "cs":{"type":["integer","null"],"minimum":0},
      "health":{"type":["number","null"],"minimum":0},
      "mana":{"type":["number","null"],"minimum":0}
    },"required":["game_time_seconds","champion","level","kills","deaths","assists","cs","health","mana"],"additionalProperties":false})
}

pub fn add_uncertain_fields(x: &mut Extraction) {
    let fields = [
        ("game_time_seconds", x.game_time_seconds.is_none()),
        ("champion", x.champion.is_none()),
        ("level", x.level.is_none()),
        ("kills", x.kills.is_none()),
        ("deaths", x.deaths.is_none()),
        ("assists", x.assists.is_none()),
        ("cs", x.cs.is_none()),
        ("health", x.health.is_none()),
        ("mana", x.mana.is_none()),
    ];
    x.uncertain_fields = fields
        .into_iter()
        .filter_map(|(name, missing)| missing.then_some(name.to_string()))
        .collect();
}

pub fn validate(x: &Extraction) -> Vec<String> {
    let mut e = Vec::new();
    if matches!(x.level, Some(0) | Some(19..)) {
        e.push("level outside 1..=18".into());
    }
    if x.health.is_some_and(|value| value > 100_000.0) {
        e.push("health exceeds plausible HUD range".into());
    }
    if x.mana.is_some_and(|value| value > 100_000.0) {
        e.push("mana exceeds plausible HUD range".into());
    }
    for (name, value) in [
        ("kills", x.kills),
        ("deaths", x.deaths),
        ("assists", x.assists),
    ] {
        if value.is_some_and(|value| value > 100) {
            e.push(format!("{name} exceeds plausible match total"));
        }
    }
    if x.cs.is_some_and(|value| value > 2_000) {
        e.push("cs exceeds plausible match total".into());
    }
    e
}

pub fn validate_temporal(previous: &Extraction, current: &Extraction) -> Vec<String> {
    let mut errors = Vec::new();
    for (name, before, after) in [
        (
            "game_time_seconds",
            previous.game_time_seconds,
            current.game_time_seconds,
        ),
        (
            "level",
            previous.level.map(u32::from),
            current.level.map(u32::from),
        ),
        ("kills", previous.kills, current.kills),
        ("deaths", previous.deaths, current.deaths),
        ("assists", previous.assists, current.assists),
        ("cs", previous.cs, current.cs),
    ] {
        if let (Some(before), Some(after)) = (before, after) {
            if after < before {
                errors.push(format!("{name} decreased from {before} to {after}"));
            }
        }
    }
    errors
}
