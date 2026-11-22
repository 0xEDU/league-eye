use serde::{Deserialize, Serialize};

pub const TIMELINE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameReference {
    pub video_timestamp_ms: u64,
    pub crop: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub schema_version: u32,
    pub evidence_id: String,
    pub video_fingerprint: String,
    pub detector: String,
    pub detector_version: String,
    pub frame: FrameReference,
    pub observed_kind: EvidenceKind,
    pub confidence: f64,
    pub factual_summary: String,
    pub validation_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    SceneChange,
    Death,
    Recall,
    Objective,
    Fight,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationState {
    Confirmed,
    Uncertain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub schema_version: u32,
    pub event_id: String,
    pub event_kind: EvidenceKind,
    pub start_timestamp_ms: u64,
    pub end_timestamp_ms: u64,
    pub confirmation: ConfirmationState,
    pub confidence: f64,
    pub evidence_ids: Vec<String>,
    pub factual_summary: String,
}
