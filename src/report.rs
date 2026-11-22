use crate::domain::{ConfirmationState, TimelineEvent};

pub fn markdown(events: &[TimelineEvent]) -> String {
    let mut report = String::from("# League-eye Timeline\n\n");
    if events.is_empty() {
        report.push_str("No match-wide events were detected.\n");
        return report;
    }
    for event in events {
        let timestamp = format_timestamp(event.start_timestamp_ms);
        let state = match event.confirmation {
            ConfirmationState::Confirmed => "confirmed",
            ConfirmationState::Uncertain => "uncertain",
        };
        report.push_str(&format!(
            "- {timestamp} — {:?} ({state}, {:.0}%): {} [evidence: {}]\n",
            event.event_kind,
            event.confidence * 100.0,
            event.factual_summary,
            event.evidence_ids.join(", ")
        ));
    }
    report
}

fn format_timestamp(timestamp_ms: u64) -> String {
    let seconds = timestamp_ms / 1_000;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}
