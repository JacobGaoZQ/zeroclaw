"""
Strands Agents A2A Server (with CORS support)
=============================================
A Python A2A-compatible agent powered by Strands Agents SDK and Qwen LLM.
Runs on port 9000 and can communicate with ZeroClaw agents via the A2A protocol.

Features:
- CORS enabled for cross-origin requests from ZeroClaw UI
- /a2a endpoint for ZeroClaw compatibility (Strands uses / by default)
- Qwen LLM via OpenAI-compatible API
- Tools: http_request, shell, file_read, file_write

Usage:
    export QWEN_API_KEY=<your-key>
    python strands_a2a_server.py

Optional:
    export CODESPACE_NAME=<name>   # auto-set in GitHub Codespaces
    export PORT=9000               # override default port
"""

import logging
import os

import httpx
import uvicorn
from openai import AsyncOpenAI
from starlette.applications import Starlette
from starlette.middleware.cors import CORSMiddleware
from starlette.requests import Request
from starlette.responses import Response
from starlette.routing import Mount, Route
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

# ── A2A Server with CORS and /a2a route ───────────────────────────

# Create Strands A2A server instance
a2a_server = A2AServer(
    agent=agent,
    host=HOST,
    port=PORT,
    http_url=PUBLIC_URL,
)

# Build Starlette app from A2A server (this has routes on /)
a2a_app = a2a_server.to_starlette_app()


# Proxy handler for /a2a -> /
async def a2a_proxy(request: Request) -> Response:
    """Proxy /a2a requests to / for ZeroClaw compatibility."""
    body = await request.body()
    
    # Parse and modify the request to include pat_token and location_id in the message
    try:
        import json
        body_json = json.loads(body)
        method = body_json.get("method", "")
        params = body_json.get("params", {})
        
        # Log received parameters
        logger.info("=" * 60)
        logger.info("Received A2A request:")
        logger.info("  method: %s", method)
        logger.info("  params keys: %s", list(params.keys()))
        
        pat_token = params.get("pat_token")
        location_id = params.get("location_id")
        
        if pat_token:
            logger.info("  pat_token: %s", pat_token)
        if location_id:
            logger.info("  location_id: %s", location_id)
        
        # If message/send, inject parameter info into the message text
        if method == "message/send" and "message" in params:
            message = params["message"]
            parts = message.get("parts", [])
            
            # Find the text part and prepend parameter info
            param_info = ""
            if pat_token or location_id:
                param_info = "[系统提示] 收到的请求参数:\n"
                if pat_token:
                    param_info += f"- pat_token: {pat_token}\n"
                if location_id:
                    param_info += f"- location_id: {location_id}\n"
                param_info += "\n请在回复中确认已收到这些参数。\n\n"
            
            for part in parts:
                if isinstance(part, dict) and part.get("kind") == "text":
                    original_text = part.get("text", "")
                    logger.info("  original message: %s", original_text)
                    part["text"] = param_info + original_text
                    logger.info("  modified message: %s", part["text"])
                    break
            
            # Remove pat_token and location_id from params (they're now in the message)
            params.pop("pat_token", None)
            params.pop("location_id", None)
        
        logger.info("=" * 60)
        
        # Re-serialize the modified body
        body = json.dumps(body_json).encode()
    except Exception as e:
        logger.warning("Failed to parse/modify request body: %s", e)
    
    async with httpx.AsyncClient() as client:
        try:
            response = await client.post(
                f"http://localhost:{PORT}/",
                content=body,
                headers={
                    "Content-Type": request.headers.get("Content-Type", "application/json"),
                    "Authorization": request.headers.get("Authorization", ""),
                },
                timeout=60.0,
            )
            return Response(
                content=response.content,
                status_code=response.status_code,
                headers={
                    "Content-Type": "application/json",
                    "Access-Control-Allow-Origin": "*",
                },
            )
        except Exception as e:
            logger.error("Proxy error: %s", e)
            return Response(
                content=f'{{"jsonrpc":"2.0","error":{{"code":-32000,"message":"Proxy error: {e}"}}}}'.encode(),
                status_code=500,
                media_type="application/json",
            )


# Create parent app with explicit routes
# Order matters: /a2a must be before the catch-all mount
routes = [
    Route("/a2a", a2a_proxy, methods=["POST"]),
    Mount("/", app=a2a_app),
]

app = Starlette(routes=routes)

# Add CORS middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Entrypoint ────────────────────────────────────────────────────

if __name__ == "__main__":
    logger.info("Starting Strands A2A server on %s:%d", HOST, PORT)
    logger.info("Agent card: %s/.well-known/agent-card.json", PUBLIC_URL)
    logger.info("CORS enabled for cross-origin requests")
    logger.info("ZeroClaw compatible endpoint: %s/a2a", PUBLIC_URL)
    uvicorn.run(app, host=HOST, port=PORT)
