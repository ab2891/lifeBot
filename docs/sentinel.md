# Lifebot Sentinel

> **SAFETY DISCLAIMER**: Sentinel is an assistive safety layer only. It is **NOT** a replacement for active lifeguard surveillance. All alerts require human acknowledgment and decision-making.

## What Sentinel Is

Sentinel is a human-in-the-loop surveillance/alert module integrated into the Lifebot platform. It monitors pool zones for possible safety events (prolonged immobility, unresponsive swimmers, motion timeouts) and escalates suspected incidents to currently on-duty deck supervisors.

## How It Works

### Detection Pipeline

1. A **detection provider** analyzes camera/zone data and produces events
2. Events are scored for **confidence** and **severity** based on duration, zone type, and confidence level
3. Alerts are created with tiered severity: **low** (log only), **medium** (notify supervisors), **high** (escalate with follow-up)
4. Supervisors receive notifications and can **acknowledge**, **resolve**, **dismiss as false positive**, or **escalate** alerts
5. All actions are logged for auditability

### Severity Scoring

Severity is computed as: `score = confidence × (duration / 10) × zone_multiplier`

| Zone Type | Multiplier |
|-----------|-----------|
| deep_end, diving | 1.5x |
| lap_lane | 1.2x |
| general, shallow | 1.0x |

| Score | Severity |
|-------|----------|
| ≥ 3.0 | high |
| ≥ 1.5 | medium |
| < 1.5 | low |

### Scheduling Integration

Sentinel resolves alert recipients dynamically from the active shift schedule:
- Queries `shift_assignments` for the current cycle at the relevant site
- Filters for guards with a "Supervisor" role
- Does not hardcode recipients

## Running Locally in Demo Mode

Sentinel works entirely in demo mode with mock data:

```bash
# Web server mode
cargo run -p lifebot-web-server

# Desktop app mode
cargo tauri dev
```

The demo seeds:
- One camera per pool (mock stream URL)
- Three zones per pool (Deep End, Shallow End, Lap Lanes) with configurable immobility thresholds
- No real camera infrastructure required

Use the **Sentinel tab** in the app to:
- Click "Simulate detection event" to generate test events with configurable zone, type, confidence, and duration
- Acknowledge, resolve, dismiss, or escalate alerts
- View event history

The assistant also supports Sentinel queries:
- "Show active Sentinel alerts"
- "Simulate an unresponsive swimmer in the deep end"
- "Who are the current supervisors for this pool?"
- "Show Sentinel event history"

## What Is Mocked

- **Camera feeds**: All cameras use `mock://local/stream` URLs. No real video processing.
- **Detection provider**: `MockDetectionProvider` — events are generated explicitly via the simulate function, not from real CV analysis.
- **Notifications**: Use the in-app channel. Real SMS/GroupMe would use the existing messaging provider abstraction.

## Database Schema

| Table | Purpose |
|-------|---------|
| cameras | Registered camera sources per site |
| pool_zones | Monitored zones with camera mappings and thresholds |
| sentinel_events | Raw detection events with confidence/duration |
| sentinel_alerts | Tiered alerts with severity and status tracking |
| sentinel_alert_recipients | Who was notified for each alert |
| sentinel_acknowledgments | All human actions on alerts (audit trail) |
| incident_outcomes | Final resolution records |

## Future Vision Provider Integration

The detection pipeline uses a pluggable `DetectionProvider` trait:

```rust
pub trait DetectionProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn generate_detection(&self, zone: &PoolZone) -> Option<DetectionResult>;
}
```

To integrate real CV (e.g., OpenCV, a Python worker, or a cloud vision API):

1. Implement `DetectionProvider` for your backend
2. Connect it to RTSP camera streams via the `stream_url` field on each camera
3. Run detections on a background timer and call `simulate_event` with real results
4. The rest of the pipeline (severity scoring, alerting, supervisor lookup, UI) works unchanged

## Assistant Tools

| Tool | Description |
|------|-------------|
| `list_active_sentinel_alerts` | Show current active alerts |
| `acknowledge_sentinel_alert` | Mark alert as seen |
| `dismiss_sentinel_alert` | Mark as false positive |
| `explain_sentinel_event` | Get full alert detail |
| `get_current_supervisors_for_pool` | Who should receive alerts |
| `simulate_sentinel_event` | Generate test event |
| `list_sentinel_event_history` | Recent detection events |
