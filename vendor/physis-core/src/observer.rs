//! Pluggable semantic-observer interface (offline, no authority).
//!
//! A [`SemanticObserver`] turns external inputs into [`crate::ingest::CandidateClaimPacket`]s.
//! Core ships only dependency-free traits and mock/replay implementations —
//! real observers (Cognee sidecar, Cognee-RS, extractors) live behind this
//! trait outside core. Observer outputs are candidates only: they may cite
//! evidence and propose structure, but cannot assert verified truth, delete
//! history, or bypass TMS/contradictions.

use std::collections::VecDeque;

use crate::ingest::CandidateClaimPacket;

/// Opaque input handed to an observer (core does not interpret it).
///
/// Kept simple and serializable so tests and adapters can round-trip inputs
/// deterministically without pulling in runtime-specific types.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObserverInput {
    /// Source/subject identifier (uri/path/episode key).
    pub source: String,
    /// Text/content payload (may be empty for pre-structured inputs).
    #[serde(default)]
    pub content: String,
    /// Optional MIME/kind hint.
    #[serde(default)]
    pub kind: Option<String>,
}

/// A semantic observer: produces candidate claims from inputs.
///
/// Implementations must be deterministic for a fixed input+configuration where
/// possible (or record enough version/config identity for replay). Returning
/// claims must not mutate canonical world state.
pub trait SemanticObserver: Send + Sync {
    /// Stable observer id (recorded in provenance).
    fn observer_id(&self) -> &str;

    /// Extract candidate claims from a batch of inputs.
    fn ingest(&self, inputs: &[ObserverInput]) -> anyhow::Result<Vec<CandidateClaimPacket>>;
}

/// No-op observer (default/offline): produces no claims.
#[derive(Debug, Default, Clone)]
pub struct NoopObserver;

impl SemanticObserver for NoopObserver {
    fn observer_id(&self) -> &str {
        "noop"
    }
    fn ingest(&self, _inputs: &[ObserverInput]) -> anyhow::Result<Vec<CandidateClaimPacket>> {
        Ok(Vec::new())
    }
}

/// Deterministic mock observer for tests and replay.
///
/// Modes:
/// - **fixed**: returns a configured set of (normalized) packets for any input.
/// - **scripted**: returns successive batches per `ingest` call (FIFO), useful
///   for replaying recorded observer output without the observer runtime.
#[derive(Debug, Default, Clone)]
pub struct MockObserver {
    id: String,
    fixed: Vec<CandidateClaimPacket>,
    scripted: VecDeque<Vec<CandidateClaimPacket>>,
}

impl MockObserver {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            fixed: Vec::new(),
            scripted: VecDeque::new(),
        }
    }

    /// Return these packets (normalized) on every ingest call.
    pub fn with_fixed_packets(mut self, packets: Vec<CandidateClaimPacket>) -> Self {
        self.fixed = packets.into_iter().map(|p| p.normalize()).collect();
        self
    }

    /// Append a batch to be returned on the next ingest call (in order).
    pub fn push_scripted_batch(&mut self, packets: Vec<CandidateClaimPacket>) {
        self.scripted
            .push_back(packets.into_iter().map(|p| p.normalize()).collect());
    }

    pub fn with_scripted_batches(mut self, batches: Vec<Vec<CandidateClaimPacket>>) -> Self {
        for b in batches {
            self.push_scripted_batch(b);
        }
        self
    }
}

impl SemanticObserver for MockObserver {
    fn observer_id(&self) -> &str {
        &self.id
    }

    fn ingest(&self, _inputs: &[ObserverInput]) -> anyhow::Result<Vec<CandidateClaimPacket>> {
        // For &self we cannot pop scripted; fixed mode is the pure replay path.
        // Scripted consumption is provided via `ingest_mut`.
        Ok(self.fixed.clone())
    }
}

impl MockObserver {
    /// Consume the next scripted batch if present, else fall back to fixed.
    pub fn ingest_mut(
        &mut self,
        inputs: &[ObserverInput],
    ) -> anyhow::Result<Vec<CandidateClaimPacket>> {
        if let Some(batch) = self.scripted.pop_front() {
            Ok(batch)
        } else {
            self.ingest(inputs)
        }
    }
}

/// Record the packets produced by an inner observer (deterministic replay log).
#[derive(Debug, Default)]
pub struct RecordingObserver<O> {
    inner: O,
    pub recorded: std::sync::Mutex<Vec<(Vec<ObserverInput>, Vec<CandidateClaimPacket>)>>,
}

impl<O> RecordingObserver<O> {
    pub fn new(inner: O) -> Self {
        Self {
            inner,
            recorded: std::sync::Mutex::new(Vec::new()),
        }
    }
    pub fn into_inner(self) -> O {
        self.inner
    }
}

impl<O: SemanticObserver> SemanticObserver for RecordingObserver<O> {
    fn observer_id(&self) -> &str {
        self.inner.observer_id()
    }
    fn ingest(&self, inputs: &[ObserverInput]) -> anyhow::Result<Vec<CandidateClaimPacket>> {
        let out = self.inner.ingest(inputs)?;
        if let Ok(mut rec) = self.recorded.lock() {
            rec.push((inputs.to_vec(), out.clone()));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{
        CandidateClaimPacket, ClaimPayload, ClaimSource, EntityMention, ExtractorInfo,
    };

    fn pkt(label: &str) -> CandidateClaimPacket {
        CandidateClaimPacket {
            id: "claim:auto".into(),
            intake_id: Some("i1".into()),
            source: ClaimSource {
                uri: "s".into(),
                span: Some("0..1".into()),
                kind: None,
            },
            extractor: ExtractorInfo::new("mock"),
            asserted_at: None,
            observed_at: None,
            raw_ref: None,
            confidence: Some(0.5),
            payload: ClaimPayload::Entity(EntityMention {
                label: label.into(),
                refers_to: None,
                concept: None,
            }),
            lossy_fields: vec![],
            extras: Default::default(),
        }
    }

    #[test]
    fn noop_and_fixed_mock_are_deterministic() {
        let noop = NoopObserver;
        assert!(noop.ingest(&[]).unwrap().is_empty());

        let m = MockObserver::new("m").with_fixed_packets(vec![pkt("a")]);
        let a = m.ingest(&[ObserverInput::default()]).unwrap();
        let b = m.ingest(&[ObserverInput::default()]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 1);
        assert!(a[0].id.starts_with("claim:"));
    }

    #[test]
    fn scripted_mock_replays_in_order() {
        let mut m = MockObserver::new("m")
            .with_scripted_batches(vec![vec![pkt("a")], vec![pkt("b"), pkt("c")]]);
        let a = m.ingest_mut(&[]).unwrap();
        let b = m.ingest_mut(&[]).unwrap();
        let c = m.ingest_mut(&[]).unwrap(); // exhausted -> fixed (empty)
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 2);
        assert!(c.is_empty());
    }

    #[test]
    fn recording_observer_captures_output() {
        let inner = MockObserver::new("m").with_fixed_packets(vec![pkt("a")]);
        let rec = RecordingObserver::new(inner);
        let out = rec
            .ingest(&[ObserverInput {
                source: "x".into(),
                content: "y".into(),
                kind: None,
            }])
            .unwrap();
        assert_eq!(out.len(), 1);
        let log = rec.recorded.lock().unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].1, out);
    }
}
