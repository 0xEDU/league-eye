use crate::domain::{
    ConfirmationState, EvidenceKind, EvidenceRecord, TIMELINE_SCHEMA_VERSION, TimelineEvent,
};

pub fn build(evidence: &[EvidenceRecord], merge_window_ms: u64) -> Vec<TimelineEvent> {
    let mut ordered = evidence.to_vec();
    ordered.sort_by_key(|record| record.frame.video_timestamp_ms);
    let mut events = Vec::new();
    for record in ordered {
        if matches!(
            record.observed_kind,
            EvidenceKind::SceneChange | EvidenceKind::Unknown
        ) {
            continue;
        }
        let can_merge = events.last().is_some_and(|event: &TimelineEvent| {
            event.event_kind == record.observed_kind
                && record
                    .frame
                    .video_timestamp_ms
                    .saturating_sub(event.end_timestamp_ms)
                    <= merge_window_ms
        });
        if can_merge {
            let event = events.last_mut().expect("event exists");
            event.end_timestamp_ms = record.frame.video_timestamp_ms;
            event.confidence = event.confidence.max(record.confidence);
            event.evidence_ids.push(record.evidence_id);
            continue;
        }
        let ordinal = events.len() + 1;
        events.push(TimelineEvent {
            schema_version: TIMELINE_SCHEMA_VERSION,
            event_id: format!("event-{ordinal:05}"),
            event_kind: record.observed_kind.clone(),
            start_timestamp_ms: record.frame.video_timestamp_ms,
            end_timestamp_ms: record.frame.video_timestamp_ms,
            confirmation: if record.confidence >= 0.85 {
                ConfirmationState::Confirmed
            } else {
                ConfirmationState::Uncertain
            },
            confidence: record.confidence,
            evidence_ids: vec![record.evidence_id],
            factual_summary: record.factual_summary,
        });
    }
    events
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::domain::{EvidenceKind, EvidenceRecord, FrameReference, TIMELINE_SCHEMA_VERSION};

    fn evidence(id: &str, timestamp: u64) -> EvidenceRecord {
        EvidenceRecord {
            schema_version: TIMELINE_SCHEMA_VERSION,
            evidence_id: id.into(),
            video_fingerprint: "video".into(),
            detector: "test".into(),
            detector_version: "v1".into(),
            frame: FrameReference {
                video_timestamp_ms: timestamp,
                crop: None,
            },
            observed_kind: EvidenceKind::Fight,
            confidence: 0.6,
            factual_summary: "Visible fight candidate.".into(),
            validation_notes: vec![],
        }
    }

    #[test]
    fn merges_nearby_same_kind_evidence() {
        let events = build(&[evidence("a", 1_000), evidence("b", 1_500)], 1_000);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].evidence_ids, ["a", "b"]);
    }

    #[test]
    fn excludes_unknown_evidence_from_the_timeline() {
        let mut unknown = evidence("unknown", 1_000);
        unknown.observed_kind = EvidenceKind::Unknown;
        assert!(build(&[unknown], 1_000).is_empty());
    }
}
