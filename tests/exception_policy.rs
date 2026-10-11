use shipment_agent_failure_log::shipment_loop::{
    decide, LoopDecision, ShipmentEvent, ShipmentFailure,
};

fn shipment(attempt: u8) -> ShipmentEvent {
    ShipmentEvent {
        event_id: "evt-7".to_owned(),
        shipment_id: "SHP-7".to_owned(),
        attempt,
        proof: None,
    }
}

#[test]
fn routes_missing_proof_to_manual_review() {
    assert_eq!(
        decide(&shipment(1), &ShipmentFailure::MissingProof),
        LoopDecision::ManualReview
    );
}

#[test]
fn caps_transient_carrier_retries() {
    assert_eq!(
        decide(&shipment(2), &ShipmentFailure::CarrierUnavailable),
        LoopDecision::Retry
    );
    assert_eq!(
        decide(&shipment(3), &ShipmentFailure::CarrierUnavailable),
        LoopDecision::Stop
    );
}
