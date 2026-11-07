//! Temporal validity — when is a statement true?
//!
//! Physis must distinguish:
//!
//!   X is true
//!   X was true at time T
//!   X is true during interval [start, end]
//!
//! This module provides the primitives.

use serde::{Deserialize, Serialize};

/// A point in time or an interval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalValidity {
    /// When this becomes valid. None = valid from the beginning of time.
    pub valid_from: Option<chrono::DateTime<chrono::Utc>>,
    /// When this ceases to be valid. None = still valid / permanent.
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Optional event that triggered the validity (e.g. "startup", "intervention").
    #[serde(default)]
    pub trigger: Option<String>,
    /// System-recorded invalidation time (Graphiti `expired_at` leg): when the
    /// record itself was superseded, distinct from `valid_until` (when the
    /// claim stopped being true). None = never invalidated. **P2/G6 reads
    /// this leg**: a point-in-time query excludes records past
    /// `expired_at` while the audit trail keeps them replayable (invalidate,
    /// don't delete).
    #[serde(default)]
    pub expired_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TemporalValidity {
    /// Permanent validity (no time bounds).
    pub fn permanent() -> Self {
        Self {
            valid_from: None,
            valid_until: None,
            trigger: None,
            expired_at: None,
        }
    }

    /// Valid from a specific point onward.
    pub fn from(instant: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            valid_from: Some(instant),
            valid_until: None,
            trigger: None,
            expired_at: None,
        }
    }

    /// Valid during an interval.
    pub fn during(
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            valid_from: Some(start),
            valid_until: Some(end),
            trigger: None,
            expired_at: None,
        }
    }

    /// Is this statement valid at the given instant?
    ///
    /// P2/G6: read all three legs. The claim's own interval
    /// (`valid_from` / `valid_until`) says when the statement was true; the
    /// system leg (`expired_at`) says when the *record* was superseded. A
    /// point-in-time query honours both — history keeps the old record, the
    /// validity surface does not.
    pub fn is_valid_at(&self, when: chrono::DateTime<chrono::Utc>) -> bool {
        if let Some(from) = self.valid_from {
            if when < from {
                return false;
            }
        }
        if let Some(until) = self.valid_until {
            if when >= until {
                return false;
            }
        }
        if let Some(expired) = self.expired_at {
            if when >= expired {
                return false;
            }
        }
        true
    }

    /// Is this statement valid now?
    pub fn is_current(&self) -> bool {
        self.is_valid_at(chrono::Utc::now())
    }

    /// Does this overlap with another validity window?
    pub fn overlaps(&self, other: &TemporalValidity) -> bool {
        let start_a = self
            .valid_from
            .unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap());
        let end_a = self.valid_until.unwrap_or_else(|| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(i64::MAX, 0).unwrap()
        });
        let start_b = other
            .valid_from
            .unwrap_or_else(|| chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0).unwrap());
        let end_b = other.valid_until.unwrap_or_else(|| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(i64::MAX, 0).unwrap()
        });
        start_a < end_b && start_b < end_a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanent_is_always_valid() {
        let t = TemporalValidity::permanent();
        assert!(t.is_current());
    }

    #[test]
    fn interval_validity() {
        let start = chrono::Utc::now();
        let end = start + chrono::Duration::hours(1);
        let t = TemporalValidity::during(start, end);
        assert!(t.is_valid_at(start));
        assert!(t.is_valid_at(start + chrono::Duration::minutes(30)));
        assert!(!t.is_valid_at(end));
    }
}
