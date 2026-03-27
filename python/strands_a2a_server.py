"""
Strands Agents A2A Server
=========================
A Python A2A-compatible agent powered by Strands Agents SDK and Qwen LLM.
Runs on port 9000 and can communicate with ZeroClaw agents via the A2A protocol.

Usage:
    export QWEN_API_KEY=<your-key>
    python strands_a2a_server.py

Optional:
    export CODESPACE_NAME=<name>   # auto-set in GitHub Codespaces
    export PORT=9000               # override default port
"""

import logging
import os

from openai import AsyncOpenAI
from strands import Agent
from strands.models.openai import OpenAIModel
from strands.multiagent.a2a import A2AServer
from strands_tools import file_read, file_write, http_request, shell

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
)
logger = logging.getLogger(__name__)

# ── Configuration ─────────────────────────────────────────────────

API_KEY = os.environ.get("QWEN_API_KEY") or os.environ.get("API_KEY")
if not API_KEY:
    raise RuntimeError(
        "QWEN_API_KEY (or API_KEY) environment variable is required.\n"
        "Example: export QWEN_API_KEY=sk-xxxx"
    )

PORT = int(os.environ.get("PORT", "9000"))
HOST = "0.0.0.0"

# Derive the public URL for the A2A agent card.
# In Codespaces, CODESPACE_NAME is automatically set.
CODESPACE_NAME = os.environ.get("CODESPACE_NAME", "")
if CODESPACE_NAME:
    PUBLIC_URL = f"https://{CODESPACE_NAME}-{PORT}.app.github.dev"
else:
    PUBLIC_URL = f"http://localhost:{PORT}"

logger.info("Public URL: %s", PUBLIC_URL)

# ── LLM Model ────────────────────────────────────────────────────

qwen_client = AsyncOpenAI(
    api_key=API_KEY,
    base_url="https://dashscope.aliyuncs.com/compatible-mode/v1",
)

model = OpenAIModel(
    client=qwen_client,
    model_id="qwen-plus",
)

# ── Agent ─────────────────────────────────────────────────────────

agent = Agent(
    model=model,
    name="Strands-Qwen Agent",
    description="A Strands Agents A2A node powered by Qwen, for cross-framework A2A interop testing with ZeroClaw.",
    tools=[http_request, shell, file_read, file_write],
    system_prompt=(
        "You are a helpful AI assistant powered by Qwen via the Strands Agents framework. "
        "You can execute shell commands, read/write files, and make HTTP requests. "
        "You communicate with other agents using the A2A protocol."
    ),
)

# ── A2A Server ────────────────────────────────────────────────────

server = A2AServer(
    agent=agent,
    host=HOST,
    port=PORT,
    http_url=PUBLIC_URL,
)

# ── Entrypoint ────────────────────────────────────────────────────

if __name__ == "__main__":
    logger.info("Starting Strands A2A server on %s:%d", HOST, PORT)
    logger.info("Agent card: %s/.well-known/agent-card.json", PUBLIC_URL)
    server.serve()
