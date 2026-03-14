use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::value_objects::*;

/// Base domain event — immutable, timestamped, unique.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainEvent {
    pub id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: EventKind,
}

impl DomainEvent {
    pub fn new(kind: EventKind) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            occurred_at: Utc::now(),
            kind,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    TransformApplied {
        file_path: String,
        pattern_id: PatternId,
        confidence: Confidence,
    },
    PatternMatched {
        file_path: String,
        pattern_id: PatternId,
        span_start: usize,
        span_end: usize,
    },
    FileAnalysed {
        file_path: String,
        language: Language,
        constructs_found: usize,
    },
    RepoScanCompleted {
        repo_path: String,
        files_scanned: usize,
        patterns_matched: usize,
    },
    TransformRejected {
        file_path: String,
        pattern_id: PatternId,
        reason: String,
    },
}
