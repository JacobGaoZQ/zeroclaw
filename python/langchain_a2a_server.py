"""
LangChain A2A Server (with python-a2a)
======================================
A Python A2A-compatible agent powered by LangChain and Qwen LLM.
Uses python-a2a library for A2A protocol implementation.
Runs on port 9001 and can communicate with ZeroClaw agents via the A2A protocol.

Usage:
    export QWEN_API_KEY=<your-key>
    python langchain_a2a_server.py
"""

import json
import logging
import os
import subprocess
import uuid
from datetime import datetime
from typing import Any, Dict, List, Optional

import httpx
import uvicorn
from langchain_core.tools import Tool
from langchain_core.messages import HumanMessage, SystemMessage
from langchain_openai import ChatOpenAI
from starlette.applications import Starlette
from starlette.middleware.cors import CORSMiddleware
from starlette.requests import Request
from starlette.responses import JSONResponse, Response
from starlette.routing import Route

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

PORT = int(os.environ.get("PORT", "9001"))
HOST = "0.0.0.0"

CODESPACE_NAME = os.environ.get("CODESPACE_NAME", "")
if CODESPACE_NAME:
    PUBLIC_URL = f"https://{CODESPACE_NAME}-{PORT}.app.github.dev"
else:
    PUBLIC_URL = f"http://localhost:{PORT}"

logger.info("Public URL: %s", PUBLIC_URL)

# ── LangChain LLM ────────────────────────────────────────────────

llm = ChatOpenAI(
    api_key=API_KEY,
    base_url="https://dashscope.aliyuncs.com/compatible-mode/v1",
    model="qwen-plus",
    temperature=0.7,
)

# ── Tools ─────────────────────────────────────────────────────────

def http_request_tool(url: str, method: str = "GET", body: str = "") -> str:
    """Make HTTP requests to external APIs."""
    try:
        if method.upper() == "GET":
            response = httpx.get(url, timeout=30.0)
        elif method.upper() == "POST":
            response = httpx.post(url, content=body, timeout=30.0)
        elif method.upper() == "PUT":
            response = httpx.put(url, content=body, timeout=30.0)
        elif method.upper() == "DELETE":
            response = httpx.delete(url, timeout=30.0)
        else:
            return f"Unsupported HTTP method: {method}"
        return f"Status: {response.status_code}\nBody: {response.text[:2000]}"
    except Exception as e:
        return f"HTTP request failed: {str(e)}"


def shell_tool(command: str) -> str:
    """Execute shell commands. Use with caution!"""
    try:
        result = subprocess.run(
            command,
            shell=True,
            capture_output=True,
            text=True,
            timeout=60.0,
        )
        output = result.stdout if result.stdout else result.stderr
        return output[:2000] if output else "Command executed successfully (no output)"
    except subprocess.TimeoutExpired:
        return "Command timed out after 60 seconds"
    except Exception as e:
        return f"Shell command failed: {str(e)}"


def file_read_tool(path: str) -> str:
    """Read content from a file."""
    try:
        if not path.startswith("/tmp/"):
            return "Error: For security, only files in /tmp/ can be read"
        with open(path, "r", encoding="utf-8") as f:
            content = f.read()
        return content[:5000] if content else "File is empty"
    except Exception as e:
        return f"File read failed: {str(e)}"


def file_write_tool(path: str, content: str) -> str:
    """Write content to a file."""
    try:
        if not path.startswith("/tmp/"):
            return "Error: For security, only files in /tmp/ can be written"
        with open(path, "w", encoding="utf-8") as f:
            f.write(content)
        return f"Successfully wrote to {path}"
    except Exception as e:
        return f"File write failed: {str(e)}"


# Create LangChain tools
langchain_tools = [
    Tool(
        name="http_request",
        func=lambda x: http_request_tool(**json.loads(x)) if x.startswith("{") else http_request_tool(x),
        description="Make HTTP requests. Input is JSON with 'url', optional 'method' (GET/POST/PUT/DELETE), optional 'body'.",
    ),
    Tool(
        name="shell",
        func=shell_tool,
        description="Execute shell commands. Input is the command string to execute.",
    ),
    Tool(
        name="file_read",
        func=file_read_tool,
        description="Read a file. Input is the file path (must start with /tmp/).",
    ),
    Tool(
        name="file_write",
        func=lambda x: file_write_tool(**json.loads(x)) if x.startswith("{") else "Error: Input must be JSON with 'path' and 'content'",
        description="Write to a file. Input is JSON with 'path' (must start with /tmp/) and 'content'.",
    ),
]

# Bind tools to LLM
llm_with_tools = llm.bind_tools(langchain_tools)

# ── Agent Card ────────────────────────────────────────────────────

AGENT_CARD = {
    "name": "LangChain-Qwen Agent",
    "description": "A LangChain A2A node powered by Qwen and python-a2a, for cross-framework A2A interop testing with ZeroClaw.",
    "url": PUBLIC_URL,
    "version": "1.0.0",
    "protocol_version": "0.3.0",
    "preferred_transport": "JSONRPC",
    "authentication": None,
    "capabilities": {
        "streaming": False,
        "pushNotifications": False,
        "google_a2a_compatible": True,
        "parts_array_format": True
    },
    "default_input_modes": ["text/plain"],
    "default_output_modes": ["text/plain"],
    "skills": [
        {
            "id": "http_request",
            "name": "HTTP Request",
            "description": "Make HTTP requests to external APIs",
            "tags": [],
            "examples": [],
            "input_modes": ["text/plain"],
            "output_modes": ["text/plain"]
        },
        {
            "id": "shell",
            "name": "Shell Command",
            "description": "Execute shell commands",
            "tags": [],
            "examples": [],
            "input_modes": ["text/plain"],
            "output_modes": ["text/plain"]
        },
        {
            "id": "file_read",
            "name": "File Read",
            "description": "Read files from /tmp/ directory",
            "tags": [],
            "examples": [],
            "input_modes": ["text/plain"],
            "output_modes": ["text/plain"]
        },
        {
            "id": "file_write",
            "name": "File Write",
            "description": "Write files to /tmp/ directory",
            "tags": [],
            "examples": [],
            "input_modes": ["text/plain"],
            "output_modes": ["text/plain"]
        }
    ],
    "provider": None,
    "documentation_url": None
}

# ── Task Storage ──────────────────────────────────────────────────

tasks: Dict[str, Dict] = {}

# ── HTTP Handlers ─────────────────────────────────────────────────

async def agent_card_handler(request: Request) -> JSONResponse:
    """Handle GET /.well-known/agent-card.json"""
    return JSONResponse(AGENT_CARD)


def extract_text_from_message(message_data: Dict) -> str:
    """Extract text from message parts."""
    parts = message_data.get("parts", [])
    texts = []
    for part in parts:
        if isinstance(part, dict):
            if part.get("type") == "text":
                texts.append(part.get("text", ""))
            elif "text" in part:
                texts.append(part["text"])
    return " ".join(texts) if texts else ""


async def process_message(user_input: str) -> str:
    """Process user message with LangChain."""
    messages = [
        SystemMessage(content="""You are a helpful AI assistant powered by Qwen via the LangChain framework.
You can execute shell commands, read/write files, and make HTTP requests.
You communicate with other agents using the A2A protocol.

Available tools:
- http_request: Make HTTP requests to external APIs
- shell: Execute shell commands (use with caution)
- file_read: Read files from /tmp/ directory
- file_write: Write files to /tmp/ directory

Respond naturally to the user's request."""),
        HumanMessage(content=user_input),
    ]

    response = await llm_with_tools.ainvoke(messages)
    return response.content


async def a2a_handler(request: Request) -> Response:
    """Handle POST /a2a - A2A JSON-RPC endpoint"""
    try:
        body = await request.json()
        logger.info("A2A request: %s", body.get("method", "unknown"))

        method = body.get("method", "")
        params = body.get("params", {})
        request_id = body.get("id", 1)

        if method == "message/send":
            message_data = params.get("message", {})
            user_input = extract_text_from_message(message_data)

            if not user_input:
                return JSONResponse({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {"code": -32602, "message": "Invalid params: no text content"},
                })

            task_id = params.get("contextId") or str(uuid.uuid4())

            # Process the message
            try:
                output = await process_message(user_input)

                # Store task result
                tasks[task_id] = {
                    "state": "completed",
                    "output": output,
                    "timestamp": datetime.utcnow().isoformat(),
                }

                response = {
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "result": {
                        "id": task_id,
                        "task_id": task_id,
                        "status": "completed",
                        "message": {
                            "role": "agent",
                            "parts": [{"type": "text", "text": output}],
                        },
                    },
                }
                return JSONResponse(response)

            except Exception as e:
                logger.error("Processing error: %s", e)
                return JSONResponse({
                    "jsonrpc": "2.0",
                    "id": request_id,
                    "error": {"code": -32000, "message": f"Processing error: {str(e)}"},
                })

        elif method == "tasks/get":
            # Support both 'task_id' and 'id' parameter names
            task_id = params.get("task_id") or params.get("id", "")
            logger.info("tasks/get request - task_id: '%s', params: %s", task_id, params)
            task = tasks.get(task_id, {"state": "unknown"})
            logger.info("tasks/get response - found: %s, known tasks: %s", task_id in tasks, list(tasks.keys())[:5])

            response = {
                "jsonrpc": "2.0",
                "id": request_id,
                "result": {
                    "task_id": task_id,
                    "state": task.get("state", "unknown"),
                    "status": {"state": task.get("state", "unknown")},
                },
            }
            return JSONResponse(response)

        else:
            return JSONResponse({
                "jsonrpc": "2.0",
                "id": request_id,
                "error": {"code": -32601, "message": f"Method not found: {method}"},
            })

    except Exception as e:
        logger.error("A2A handler error: %s", e)
        return JSONResponse({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {"code": -32000, "message": f"Internal error: {str(e)}"},
        })


# Create Starlette app
routes = [
    Route("/.well-known/agent-card.json", agent_card_handler, methods=["GET"]),
    Route("/a2a", a2a_handler, methods=["POST"]),
]

app = Starlette(routes=routes)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── Entrypoint ────────────────────────────────────────────────────

if __name__ == "__main__":
    logger.info("Starting LangChain A2A server on %s:%d", HOST, PORT)
    logger.info("Agent card: %s/.well-known/agent-card.json", PUBLIC_URL)
    logger.info("CORS enabled for cross-origin requests")
    logger.info("ZeroClaw compatible endpoint: %s/a2a", PUBLIC_URL)
    uvicorn.run(app, host=HOST, port=PORT)
