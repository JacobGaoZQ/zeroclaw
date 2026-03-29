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
from datetime import datetime
from typing import Any, Dict

import httpx
from langchain_core.tools import Tool
from langchain_core.messages import HumanMessage, SystemMessage
from langchain_openai import ChatOpenAI

from python_a2a import A2AServer, run_server, AgentCard
from python_a2a.models.agent import AgentSkill

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


# ── LangChain Agent ───────────────────────────────────────────────

llm = ChatOpenAI(
    api_key=API_KEY,
    base_url="https://dashscope.aliyuncs.com/compatible-mode/v1",
    model="qwen-plus",
    temperature=0.7,
)

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


# ── A2A Server ────────────────────────────────────────────────────

class LangChainA2AServer(A2AServer):
    """LangChain A2A Server implementation using python-a2a."""

    def __init__(self):
        # Create agent card
        agent_card = AgentCard(
            name="LangChain-Qwen Agent",
            description="A LangChain A2A node powered by Qwen and python-a2a, for cross-framework A2A interop testing with ZeroClaw.",
            url=PUBLIC_URL,
            version="1.0.0",
            capabilities={
                "streaming": False,
                "pushNotifications": False,
            },
            skills=[
                AgentSkill(
                    id="http_request",
                    name="HTTP Request",
                    description="Make HTTP requests to external APIs",
                ),
                AgentSkill(
                    id="shell",
                    name="Shell Command",
                    description="Execute shell commands",
                ),
                AgentSkill(
                    id="file_read",
                    name="File Read",
                    description="Read files from /tmp/ directory",
                ),
                AgentSkill(
                    id="file_write",
                    name="File Write",
                    description="Write files to /tmp/ directory",
                ),
            ],
        )
        super().__init__(agent_card=agent_card)
        self.tasks: Dict[str, Dict] = {}

    async def process_message(self, user_input: str) -> str:
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

    def handle_task(self, task):
        """Handle A2A task."""
        from python_a2a.models import TaskStatus, TaskState, Message, TextContent

        # Get input from task
        user_input = ""
        if hasattr(task, 'message') and task.message:
            msg = task.message
            if hasattr(msg, 'content') and msg.content:
                content = msg.content
                # Handle TextContent
                if hasattr(content, 'text'):
                    user_input = content.text
                elif hasattr(content, 'content') and isinstance(content.content, str):
                    user_input = content.content
                else:
                    user_input = str(content)

        logger.info("Received input: %s", user_input[:100] if user_input else "(empty)")

        if not user_input:
            task.status = TaskStatus(state=TaskState.FAILED, message="No input provided")
            return task

        # Process synchronously (python-a2a handles async internally)
        import asyncio
        try:
            output = asyncio.run(self.process_message(user_input))
            task.status = TaskStatus(state=TaskState.COMPLETED)
            # Create response message
            task.message = Message(
                role="agent",
                content=TextContent(text=output)
            )
        except Exception as e:
            logger.error("Processing error: %s", e)
            task.status = TaskStatus(state=TaskState.FAILED, message=str(e))

        return task


# ── Entrypoint ────────────────────────────────────────────────────

if __name__ == "__main__":
    logger.info("Starting LangChain A2A server on %s:%d", HOST, PORT)
    logger.info("Agent card: %s/.well-known/agent-card.json", PUBLIC_URL)
    logger.info("ZeroClaw compatible endpoint: %s/a2a", PUBLIC_URL)

    server = LangChainA2AServer()
    run_server(server, host=HOST, port=PORT)
