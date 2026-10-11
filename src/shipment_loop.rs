use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProofOfDelivery {
    pub object_key: String,
    pub checksum: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShipmentEvent {
    pub event_id: String,
    pub shipment_id: String,
    pub attempt: u8,
    pub proof: Option<ProofOfDelivery>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShipmentFailure {
    MissingProof,
    CarrierUnavailable,
    InvalidProof(String),
}

impl fmt::Display for ShipmentFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingProof => write!(f, "proof of delivery is missing"),
            Self::CarrierUnavailable => write!(f, "carrier lookup is unavailable"),
            Self::InvalidProof(reason) => write!(f, "proof of delivery is invalid: {reason}"),
        }
    }
}

impl std::error::Error for ShipmentFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopDecision {
    Retry,
    ManualReview,
    Stop,
}

pub fn decide(event: &ShipmentEvent, failure: &ShipmentFailure) -> LoopDecision {
    match failure {
        ShipmentFailure::CarrierUnavailable if event.attempt < 3 => LoopDecision::Retry,
        ShipmentFailure::MissingProof | ShipmentFailure::InvalidProof(_) => {
            LoopDecision::ManualReview
        }
        ShipmentFailure::CarrierUnavailable => LoopDecision::Stop,
    }
}

pub fn exception_payload(event: &ShipmentEvent, failure: &ShipmentFailure) -> String {
    format!(
        "shipment_id={} event_id={} attempt={} failure={}",
        event.shipment_id, event.event_id, event.attempt, failure
    )
}
