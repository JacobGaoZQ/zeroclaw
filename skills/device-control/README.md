# A2A Device Control

Use the A2A (Agent-to-Agent) protocol to control remote devices (TV, lights, appliances, etc.) through a remote agent.

## Configuration

### Option 1: config.toml

Add the following to your `~/.zeroclaw/config.toml`:

```toml
[a2a]
enabled = true
strands_agent_url = "http://<remote-agent-host>:<port>/a2a"
client_token = "<your-a2a-api-key>"
```

### Option 2: Environment Variables (.env)

Add the following to your `.env` file or export as environment variables:

```bash
A2A_ENABLED=true
A2A_STRANDS_AGENT_URL=http://<remote-agent-host>:<port>/a2a (Test the value of the notes in the document: http://120.26.206.98:8000/a2a)
A2A_CLIENT_TOKEN=<your-a2a-api-key> (Test the value of the notes in the document: your-a2a-api-key-here)
```

> Environment variables override `config.toml` values.

### Required Fields

| Field | Config Key | Env Var | Description |
|---|---|---|---|
| Enable | `enabled` | `A2A_ENABLED` | Must be `true` to enable the A2A tool |
| Agent URL | `strands_agent_url` | `A2A_STRANDS_AGENT_URL` | URL of the remote agent that handles device control |
| Client Token | `client_token` | `A2A_CLIENT_TOKEN` | API key for authenticating A2A requests |

### Per-Call Parameters

The `pat_token` is passed as a parameter when the agent calls the `a2a` tool, not from config. This allows different tokens to be used per invocation.

## Example Skill

A ready-to-use skill for device control is available at:

```
skills/device-control/SKILL.md
```

This skill instructs the agent to use the `a2a` tool with `action='send'` when the user requests device control (e.g., "turn on the TV", "set lights to 50%").

## Usage

Once configured and the skill is loaded, just speak naturally to the agent:

- "turn on the TV"

The agent will call the `a2a` tool with `action='send'` and forward your request to the remote agent.
