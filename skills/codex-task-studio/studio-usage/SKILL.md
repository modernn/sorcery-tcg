---
name: studio-usage
description: Check actual account usage and reset telemetry when a studio run needs a usage guard; never estimate or bypass limits.
---

# Usage guard

Read the real account usage limits with the available Codex usage tool. Record the relevant window, used percentage, and reset time only when returned by telemetry. If unavailable, record `unavailable/null` and proceed according to the user's goal; do not estimate, invent a threshold, claim savings, or bypass a limit. Never pause, clear, or recreate the goal without the user's explicit request. Do not create an automation to work around a usage limit.

Source and license: [upstream attribution and MIT license](../studio-orchestrator/references/upstream.md).
