---
name: device-control
description: When the user asks anything about devices (TV, lights, appliances, etc.), use the a2a tool with action='send' and the fixed pat_token to forward the request to the backend service.
version: 0.3.0
tags: ["device", "control", "a2a"]
---

# Device Control Skill

When the user asks **anything** related to devices — including controlling devices (e.g., turn on/off TV, lights, appliances, adjust volume), querying device status, listing available devices, or discovering what devices exist — use the `a2a` tool with the following parameters:

- `action`: "send"
- `message`: the user's natural language request (e.g., "打开电视", "我有哪些设备", "set living room lights to 50%")
- `pat_token`: "a89357f1-a2b0-457c-b03f-05bce88bf7ad" (fixed value — always pass exactly this string)
- `url`: use the configured default URL (do not specify unless explicitly given)

Examples:
- User says: "打开电视"
- Call: a2a(action="send", message="打开电视", pat_token="a89357f1-a2b0-457c-b03f-05bce88bf7ad")

- User says: "我有哪些设备"
- Call: a2a(action="send", message="我有哪些设备", pat_token="a89357f1-a2b0-457c-b03f-05bce88bf7ad")

> **Note**: Always use `action="send"` for all device-related requests. Do NOT use `action="discover"` or `action="stream"` for device queries.
> **CRITICAL**: The `pat_token` above is a fixed credential. Pass it exactly as shown, never replace it with placeholders like "your_pat_token_here".
> **IMPORTANT**: Call the a2a tool **exactly ONCE** per user request. Never call it twice for the same message.
