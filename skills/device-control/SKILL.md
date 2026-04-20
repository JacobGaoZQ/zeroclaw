---
name: device-control
description: When performing device control (TV, lights, appliances, etc.), first retrieve the pat_token from memory using memory_recall, then use the a2a tool with action='stream' and pass the pat_token as a parameter.
version: 0.2.0
tags: ["device", "control", "a2a"]
---

# Device Control Skill

When the user asks to control any device (e.g., turn on/off TV, lights, appliances, adjust volume, etc.), follow these steps:

## Step 1: Retrieve pat_token from memory

Call `memory_recall` with query="pat_token" to retrieve the authentication token.

Example:
```
memory_recall(query="pat_token")
```

The memory will return a pat_token value like `"your-pat-token-here"`.

## Step 2: Call a2a tool with pat_token

Use the `a2a` tool with the following parameters:

- `action`: "send"
- `message`: the user's natural language request (e.g., "turn on the TV", "set living room lights to 50%")
- `pat_token`: the token retrieved from memory in Step 1
- `url`: use the configured default URL (do not specify unless explicitly given)

Example:
- User says: "turn on the TV"
- Call: memory_recall(query="pat_token") → get token "simulated-pat-token-12345"
- Call: a2a(action="send", message="turn on the TV", pat_token="simulated-pat-token-12345")

> **Note**: If memory_recall returns no pat_token, inform the user that the device control token needs to be stored in memory first.
