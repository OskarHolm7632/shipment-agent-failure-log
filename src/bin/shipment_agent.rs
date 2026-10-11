use shipment_agent_failure_log::{
    infrai::{self, InfraiClient},
    shipment_loop::{decide, exception_payload, LoopDecision, ShipmentEvent, ShipmentFailure},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = ShipmentEvent {
        event_id: "evt-pod-1042".to_owned(),
        shipment_id: "SHP-1042".to_owned(),
        attempt: 1,
        proof: None,
    };
    let failure = ShipmentFailure::MissingProof;
    let decision = decide(&event, &failure);
    let client = InfraiClient::from_env()?;

    // The stable event id makes a retried write refer to the same capture operation.
    infrai::errors::capture(
        &client,
        &exception_payload(&event, &failure),
        &event.event_id,
    )
    .await?;

    match decision {
        LoopDecision::Retry => println!("retry {}", event.shipment_id),
        LoopDecision::ManualReview => println!("manual-review {}", event.shipment_id),
        LoopDecision::Stop => println!("stop {}", event.shipment_id),
    }
    Ok(())
}
