---
name: device-control
description: When performing device control (TV, lights, appliances, etc.), use the a2a tool with action='send' and pass the user's natural language request in the message parameter.
version: 0.1.0
tags: ["device", "control", "a2a"]
---

# Device Control Skill

When the user asks to control any device (e.g., turn on/off TV, lights, appliances, adjust volume, etc.), use the `a2a` tool with the following parameters:

- `action`: "send"
- `message`: the user's natural language request (e.g., "turn on the TV", "set living room lights to 50%")
- `url`: use the configured default URL (do not specify unless explicitly given)

Example:
- User says: "turn on the TV"
- Call: a2a(action="send", message="turn on the TV")
