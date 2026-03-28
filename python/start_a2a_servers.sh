#!/bin/bash
# Start A2A Servers for ZeroClaw Testing
# ======================================
# This script starts three A2A servers for cross-framework testing:
# 1. ZeroClaw (port 8080) - Rust native implementation
# 2. Strands Agent (port 9000) - Python Strands SDK
# 3. LangChain A2A (port 9001) - Python LangChain + python-a2a

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== ZeroClaw Multi-Agent A2A Test Environment ===${NC}"
echo -e "${BLUE}Starting 3 A2A-compatible agents...${NC}"
echo ""

# Check API Key
if [ -z "$QWEN_API_KEY" ]; then
    echo -e "${RED}Error: QWEN_API_KEY environment variable is not set${NC}"
    echo "Please set it first: export QWEN_API_KEY=sk-xxxx"
    exit 1
fi

echo -e "${GREEN}✓ QWEN_API_KEY is set${NC}"

# Check CODESPACE_NAME
if [ -z "$CODESPACE_NAME" ]; then
    echo -e "${YELLOW}Warning: CODESPACE_NAME not set. Using localhost.${NC}"
    echo "This should only be used for local testing, not in Codespaces."
fi

# Function to set port visibility
set_port_public() {
    local port=$1
    local name=$2
    if command -v gh &> /dev/null && [ -n "$CODESPACE_NAME" ]; then
        echo -e "${YELLOW}  Setting $name port $port to public...${NC}"
        gh codespace ports visibility $port:public -c $CODESPACE_NAME 2>/dev/null && \
            echo -e "${GREEN}  ✓ Port $port is now public${NC}" || \
            echo -e "${YELLOW}  ⚠ Could not auto-set port $port to public${NC}"
    else
        echo -e "${YELLOW}  ⚠ Port $port: Please set to Public manually${NC}"
    fi
}

# Function to wait for service
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -s "$url" > /dev/null 2>&1; then
            return 0
        fi
        sleep 1
        attempt=$((attempt + 1))
    done
    return 1
}

# Kill existing processes
echo -e "${YELLOW}Stopping existing servers...${NC}"
pkill -f "zeroclaw.*gateway start" 2>/dev/null || true
pkill -f "strands_a2a_server" 2>/dev/null || true
pkill -f "langchain_a2a_server" 2>/dev/null || true
sleep 2
echo -e "${GREEN}✓ Existing servers stopped${NC}"

# ===============================
# 1. Start ZeroClaw
# ===============================
echo ""
echo -e "${GREEN}[1/3] Starting ZeroClaw Agent (port 8080)...${NC}"

if [ ! -f "/tmp/zeroclaw-a/config.toml" ]; then
    echo -e "${YELLOW}  Creating ZeroClaw config...${NC}"
    mkdir -p /tmp/zeroclaw-a
    cat > /tmp/zeroclaw-a/config.toml << EOF
default_provider = "qwen"
default_model = "qwen-plus"
default_temperature = 0.7
api_key = "$QWEN_API_KEY"

[gateway]
port = 8080
host = "0.0.0.0"
allow_public_bind = true
require_pairing = false

[a2a]
enabled = true
agent_name = "ZeroClaw-Agent"
description = "ZeroClaw A2A Test Agent - Main node"
public_url = "https://${CODESPACE_NAME:-localhost}-8080.app.github.dev"
bearer_token = "a2a-shared-secret"
allow_local = true

[cost]
enabled = false
EOF
fi

/workspaces/zeroclaw/target/release/zeroclaw --config-dir /tmp/zeroclaw-a gateway start > /tmp/zeroclaw-a.log 2>&1 &

if wait_for_service "http://localhost:8080/.well-known/agent-card.json" "ZeroClaw"; then
    echo -e "${GREEN}  ✓ ZeroClaw is running${NC}"
    if [ -n "$CODESPACE_NAME" ]; then
        echo -e "${BLUE}    Public URL: https://${CODESPACE_NAME}-8080.app.github.dev${NC}"
    fi
else
    echo -e "${RED}  ✗ ZeroClaw failed to start${NC}"
    tail -20 /tmp/zeroclaw-a.log
    exit 1
fi

# ===============================
# 2. Start Strands Agent
# ===============================
echo ""
echo -e "${GREEN}[2/3] Starting Strands Agent (port 9000)...${NC}"

cd /workspaces/zeroclaw/python

# Check if strands dependencies are installed
if ! python -c "import strands" 2>/dev/null; then
    echo -e "${YELLOW}  Installing Strands dependencies...${NC}"
    pip install -q -r requirements-strands.txt
fi

PORT=9000 python strands_a2a_server.py > /tmp/strands-a2a.log 2>&1 &

if wait_for_service "http://localhost:9000/.well-known/agent-card.json" "Strands"; then
    echo -e "${GREEN}  ✓ Strands Agent is running${NC}"
    if [ -n "$CODESPACE_NAME" ]; then
        echo -e "${BLUE}    Public URL: https://${CODESPACE_NAME}-9000.app.github.dev${NC}"
    fi
else
    echo -e "${RED}  ✗ Strands Agent failed to start${NC}"
    tail -20 /tmp/strands-a2a.log
    exit 1
fi

# ===============================
# 3. Start LangChain A2A Server
# ===============================
echo ""
echo -e "${GREEN}[3/3] Starting LangChain A2A Server (port 9001)...${NC}"

# Check if langchain dependencies are installed
if ! python -c "import python_a2a" 2>/dev/null; then
    echo -e "${YELLOW}  Installing LangChain dependencies...${NC}"
    pip install -q -r requirements-langchain-a2a.txt
fi

PORT=9001 python langchain_a2a_server.py > /tmp/langchain-a2a.log 2>&1 &

if wait_for_service "http://localhost:9001/.well-known/agent-card.json" "LangChain"; then
    echo -e "${GREEN}  ✓ LangChain A2A is running${NC}"
    if [ -n "$CODESPACE_NAME" ]; then
        echo -e "${BLUE}    Public URL: https://${CODESPACE_NAME}-9001.app.github.dev${NC}"
    fi
else
    echo -e "${RED}  ✗ LangChain A2A failed to start${NC}"
    tail -20 /tmp/langchain-a2a.log
    exit 1
fi

# ===============================
# Set ports to public
# ===============================
echo ""
echo -e "${GREEN}Setting ports to public...${NC}"
set_port_public 8080 "ZeroClaw"
set_port_public 9000 "Strands"
set_port_public 9001 "LangChain"

# ===============================
# Summary
# ===============================
echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  All 3 A2A Servers Started Successfully${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

if [ -n "$CODESPACE_NAME" ]; then
    echo -e "${BLUE}ZeroClaw Dashboard:${NC}"
    echo "  https://${CODESPACE_NAME}-8080.app.github.dev"
    echo ""
    echo -e "${BLUE}Strands Agent Card:${NC}"
    echo "  https://${CODESPACE_NAME}-9000.app.github.dev/.well-known/agent-card.json"
    echo ""
    echo -e "${BLUE}LangChain Agent Card:${NC}"
    echo "  https://${CODESPACE_NAME}-9001.app.github.dev/.well-known/agent-card.json"
else
    echo -e "${BLUE}ZeroClaw Dashboard:${NC}  http://localhost:8080"
    echo -e "${BLUE}Strands Agent:${NC}       http://localhost:9000"
    echo -e "${BLUE}LangChain Agent:${NC}    http://localhost:9001"
fi

echo ""
echo -e "${YELLOW}Testing Commands:${NC}"
echo "  # Test ZeroClaw"
echo "  curl http://localhost:8080/.well-known/agent-card.json"
echo ""
echo "  # Test Strands"
echo "  curl http://localhost:9000/.well-known/agent-card.json"
echo ""
echo "  # Test LangChain"
echo "  curl http://localhost:9001/.well-known/agent-card.json"
echo ""
echo "  # Test A2A send to LangChain"
echo "  curl -X POST http://localhost:9001/a2a \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"jsonrpc\":\"2.0\",\"method\":\"message/send\",\"params\":{\"message\":{\"role\":\"user\",\"parts\":[{\"type\":\"text\",\"text\":\"Hello\"}]}},\"id\":1}'"

echo ""
echo -e "${YELLOW}Logs:${NC}"
echo "  ZeroClaw:  tail -f /tmp/zeroclaw-a.log"
echo "  Strands:   tail -f /tmp/strands-a2a.log"
echo "  LangChain: tail -f /tmp/langchain-a2a.log"

echo ""
echo -e "${YELLOW}Manual Port Configuration:${NC}"
echo "  If ports are not public, open VS Code Ports panel and set:"
echo "    - 8080 (ZeroClaw)  → Public"
echo "    - 9000 (Strands)   → Public"
echo "    - 9001 (LangChain) → Public"

echo ""
echo -e "${YELLOW}Stop All Servers:${NC}"
echo "  pkill -f 'zeroclaw.*gateway start'; pkill -f 'strands_a2a_server'; pkill -f 'langchain_a2a_server'"
