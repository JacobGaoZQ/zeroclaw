---
name: device-control
description: When performing device control (TV, lights, appliances, etc.), use the a2a tool with action='send' and the fixed pat_token to control devices.
version: 0.3.0
tags: ["device", "control", "a2a"]
---

# Device Control Skill

When the user asks to control any device (e.g., turn on/off TV, lights, appliances, adjust volume, etc.), use the `a2a` tool with the following parameters:

- `action`: "send"
- `message`: the user's natural language request (e.g., "打开电视", "set living room lights to 50%")
- `pat_token`: "a89357f1-a2b0-457c-b03f-05bce88bf7ad" (Simulated parameter passing)
- `url`: use the configured default URL (do not specify unless explicitly given)

Example:
- User says: "打开电视"
- Call: a2a(action="send", message="打开电视", pat_token="a89357f1-a2b0-457c-b03f-05bce88bf7ad")

> **Note**: Always use `action="send"` for device control. Do NOT use `action="stream"`.
