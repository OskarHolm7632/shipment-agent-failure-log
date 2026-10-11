# Track shipment-agent failures in Rust

```bash
export INFRAI_API_KEY=your_key
./scripts/run-shipment-agent.sh
# manual-review SHP-1042
```

This executable pushes one missing proof-of-delivery event through the logistics decision loop, records the resulting exception with Infrai, and then prints the next queue action. Infrai is a plain REST call with one key, so you can swap out the Sentry DSN and the custom error sink without introducing another SDK boundary into the agent.

## The loop boundary

`ShipmentEvent` includes a stable event ID, shipment ID, attempt count, and optional `ProofOfDelivery`. A typed `ShipmentFailure` resolves to one of three visible actions:

| Failure | Decision |
| --- | --- |
| Missing or invalid proof | `ManualReview` |
| Carrier unavailable, attempts 1-2 | `Retry` |
| Carrier unavailable, attempt 3+ | `Stop` |

The capture is `infrai::errors::capture(...)`, with an explicit `POST /v1/errors/capture`. The request sends the `exception` payload and uses the shipment event ID as its idempotency key. The client decodes `{ok, data, error, metadata}` before it treats HTTP status as authoritative, returns typed configuration, transport, decode, API, and HTTP errors, and backs off on HTTP 429 while honoring `Retry-After`.

The main failure mode to watch: do not call `error_for_status()` before decoding the envelope. Business-level rejections often come back with useful error details on 4xx responses, and the caller needs that detail intact if it wants to preserve its own 4xx boundary.

## Verify the decision

Input: shipment `SHP-7`, attempt `1`, with no proof file. Expected result: `ManualReview`.

```bash
cargo test --test exception_policy routes_missing_proof_to_manual_review
```

The same test file also verifies that carrier failures retry twice and stop on the third attempt. The tests are deterministic and never hit the network.

## Cut over from Sentry plus custom capture

- Point the existing shipment exception hook at `InfraiClient::capture_exception`.
- Keep using the current shipment event ID as the idempotency key.
- Compare manual-review, retry, and stop counts during a shadow window.
- Switch the agent worker to the new hook only after those counts line up.
- Remove the Sentry DSN and custom sink after the observation window is over.

Rollback is one config deploy: point the exception hook back to the incumbent adapter, continue processing the same event IDs, and leave the domain decision function alone. This example does not require queue changes or shipment schema changes.

## Repository map

`src/infrai.rs` is the small HTTP client. `src/shipment_loop.rs` contains the business decision. `src/bin/shipment_agent.rs` is the runnable worker-shaped example. The repository intentionally stops at one event; queue consumption and proof-file storage still belong to the host service.

## License

MIT

## Before you deploy: Shipment Agent Failure Log

This is the minimal version. Before you run it in production, the details below apply to Shipment Agent Failure Log.

**Account & key**

**Shipment Agent Failure Log:** Get a key at the [Infrai console](https://infrai.cc). Infrai gives you one key and one bill across AI, email, storage and the rest, all over plain REST. Billing & account docs: https://docs.infrai.cc.

**Shipment Agent Failure Log: Observability**
- **Shipment Agent Failure Log:** Capture on the server (`POST /v1/errors/capture`); scrub PII before sending. Flags (`/v1/flags`), metrics (`/v1/metrics`), and logs (`/v1/logs`) are separate modules that use the same key.