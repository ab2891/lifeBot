"""
Lifebot Sentinel — CV Worker

A lightweight computer vision service that analyzes camera feeds for
possible safety events (prolonged immobility, unresponsive swimmers).

This worker exposes two HTTP endpoints:
  GET  /health    — liveness check
  POST /analyze   — analyze a camera feed for a set of zones

The worker supports two modes:
  1. Mock mode (default): Returns simulated detections for testing
  2. Live mode: Captures frames from RTSP/HTTP streams via OpenCV and
     runs motion-based analysis

Run:
  pip install -r requirements.txt
  python cv_worker.py                    # mock mode on :5050
  python cv_worker.py --live             # live mode (requires cameras)
  python cv_worker.py --port 8080        # custom port

SAFETY DISCLAIMER: This is an assistive tool only. It is NOT a replacement
for active lifeguard surveillance.
"""

import argparse
import json
import time
import logging
from typing import Optional

from flask import Flask, request, jsonify

app = Flask(__name__)
logger = logging.getLogger("cv_worker")

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------

LIVE_MODE = False

# In-memory state for motion tracking (zone_id -> last_motion_timestamp)
_zone_motion_state: dict[str, float] = {}


# ---------------------------------------------------------------------------
# Health endpoint
# ---------------------------------------------------------------------------

@app.route("/health", methods=["GET"])
def health():
    return jsonify({"status": "ok", "mode": "live" if LIVE_MODE else "mock"})


# ---------------------------------------------------------------------------
# Analysis endpoint
# ---------------------------------------------------------------------------

@app.route("/analyze", methods=["POST"])
def analyze():
    """
    Expects JSON body:
    {
        "camera_id": "cam-xxx",
        "stream_url": "rtsp://... or mock://...",
        "zones": [
            {
                "zone_id": "zone-xxx",
                "name": "Main Pool — Deep End",
                "zone_type": "deep_end",
                "immobility_threshold_secs": 12
            }
        ]
    }

    Returns:
    {
        "detections": [
            {
                "zone_id": "zone-xxx",
                "event_type": "immobility",
                "confidence": 0.82,
                "duration_secs": 18.5,
                "description": "Prolonged immobility detected..."
            }
        ]
    }
    """
    data = request.get_json(force=True)
    camera_id = data.get("camera_id", "")
    stream_url = data.get("stream_url", "")
    zones = data.get("zones", [])

    if LIVE_MODE and not stream_url.startswith("mock://"):
        detections = analyze_live(camera_id, stream_url, zones)
    else:
        detections = analyze_mock(camera_id, zones)

    return jsonify({"detections": detections})


# ---------------------------------------------------------------------------
# Mock analysis (demo/testing)
# ---------------------------------------------------------------------------

def analyze_mock(camera_id: str, zones: list[dict]) -> list[dict]:
    """
    Mock analysis: returns no detections normally.
    This keeps the system quiet unless events are triggered via the Lifebot
    simulate button. The mock CV worker acts as a "healthy but quiet" service.
    """
    # In mock mode, we don't auto-generate detections.
    # The Lifebot simulate_event function handles demo events directly.
    # This endpoint exists so the health check and pipeline work end-to-end.
    return []


# ---------------------------------------------------------------------------
# Live analysis (real cameras via OpenCV)
# ---------------------------------------------------------------------------

def analyze_live(camera_id: str, stream_url: str, zones: list[dict]) -> list[dict]:
    """
    Capture a frame (or short clip) from the stream and analyze for
    motion/immobility in each zone.

    This is a basic motion-detection approach:
    1. Capture N frames over a short window
    2. Compute frame-to-frame difference
    3. If a zone shows very low motion for longer than its threshold,
       flag an immobility event

    For production, this should be replaced with a proper pose estimation
    or drowning detection model.
    """
    try:
        import cv2
        import numpy as np
    except ImportError:
        logger.error("OpenCV not installed. Install opencv-python-headless.")
        return []

    detections = []
    cap = cv2.VideoCapture(stream_url)

    if not cap.isOpened():
        logger.warning(f"Cannot open stream: {stream_url}")
        return []

    # Capture a few frames to measure motion
    frames = []
    for _ in range(5):
        ret, frame = cap.read()
        if not ret:
            break
        gray = cv2.cvtColor(frame, cv2.COLOR_BGR2GRAY)
        gray = cv2.GaussianBlur(gray, (21, 21), 0)
        frames.append(gray)
        time.sleep(0.2)  # ~1 second total capture

    cap.release()

    if len(frames) < 2:
        return []

    # Compute average motion across frames
    total_motion = 0.0
    for i in range(1, len(frames)):
        diff = cv2.absdiff(frames[i - 1], frames[i])
        total_motion += float(np.mean(diff))
    avg_motion = total_motion / (len(frames) - 1)

    now = time.time()

    for zone in zones:
        zone_id = zone["zone_id"]
        threshold = zone.get("immobility_threshold_secs", 15)
        zone_type = zone.get("zone_type", "general")

        # Very low motion → potential immobility
        motion_threshold = 3.0  # pixel intensity difference
        if avg_motion < motion_threshold:
            # Track how long this zone has been still
            if zone_id not in _zone_motion_state:
                _zone_motion_state[zone_id] = now

            still_duration = now - _zone_motion_state[zone_id]

            if still_duration >= threshold:
                confidence = min(0.95, 0.5 + (still_duration - threshold) / 60.0)
                detections.append({
                    "zone_id": zone_id,
                    "event_type": "immobility",
                    "confidence": round(confidence, 3),
                    "duration_secs": round(still_duration, 1),
                    "description": f"Prolonged immobility detected in {zone.get('name', zone_id)} for {still_duration:.0f}s (motion level: {avg_motion:.1f})"
                })
        else:
            # Motion detected — reset timer
            _zone_motion_state.pop(zone_id, None)

    return detections


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Lifebot Sentinel CV Worker")
    parser.add_argument("--port", type=int, default=5050, help="Port to listen on")
    parser.add_argument("--live", action="store_true", help="Enable live camera analysis (requires OpenCV)")
    parser.add_argument("--debug", action="store_true", help="Enable debug logging")
    args = parser.parse_args()

    LIVE_MODE = args.live

    logging.basicConfig(
        level=logging.DEBUG if args.debug else logging.INFO,
        format="[cv_worker] %(levelname)s %(message)s"
    )

    mode_str = "LIVE (real cameras)" if LIVE_MODE else "MOCK (demo only)"
    logger.info(f"Starting CV worker on port {args.port} in {mode_str} mode")
    logger.info("Safety disclaimer: This is an assistive tool only, not a replacement for lifeguards.")

    app.run(host="0.0.0.0", port=args.port, debug=args.debug)
