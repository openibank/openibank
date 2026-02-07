#!/bin/bash
#
# ResonanceX Demo - The World's First AI-Native Trading Exchange
# "Where AI Agents Trade at the Speed of Thought"
#
# Usage:
#   ./scripts/demo.sh              # Start with default settings
#   ./scripts/demo.sh --agents 20  # Start with 20 trading agents
#   ./scripts/demo.sh --port 9999  # Start on custom port
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Banner
echo ""
echo -e "${CYAN}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}                                                                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}██████╗ ███████╗███████╗ ██████╗ ███╗   ██╗ █████╗ ███╗   ██╗${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}██╔══██╗██╔════╝██╔════╝██╔═══██╗████╗  ██║██╔══██╗████╗  ██║${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}██████╔╝█████╗  ███████╗██║   ██║██╔██╗ ██║███████║██╔██╗ ██║${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}██╔══██╗██╔══╝  ╚════██║██║   ██║██║╚██╗██║██╔══██║██║╚██╗██║${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}██║  ██║███████╗███████║╚██████╔╝██║ ╚████║██║  ██║██║ ╚████║${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}   ${GREEN}╚═╝  ╚═╝╚══════╝╚══════╝ ╚═════╝ ╚═╝  ╚═══╝╚═╝  ╚═╝╚═╝  ╚═══╝${NC}   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}                                                                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}              ${YELLOW}The World's First AI-Native Trading Exchange${NC}         ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}          ${BLUE}\"Where AI Agents Trade at the Speed of Thought\"${NC}          ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}                                                                   ${CYAN}║${NC}"
echo -e "${CYAN}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Default values
PORT=8888
AGENTS=10

# Parse command line arguments
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --port) PORT="$2"; shift ;;
        --agents) AGENTS="$2"; shift ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --port PORT     Port to run the exchange on (default: 8888)"
            echo "  --agents N      Number of demo trading agents (default: 10)"
            echo "  -h, --help      Show this help message"
            echo ""
            exit 0
            ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

# Get script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}Building ResonanceX...${NC}"
cd "$PROJECT_DIR"
cargo build -p resonancex-server --release 2>/dev/null || cargo build -p resonancex-server

echo ""
echo -e "${GREEN}Starting ResonanceX Exchange...${NC}"
echo ""
echo -e "  ${CYAN}Dashboard:${NC}  http://localhost:${PORT}"
echo -e "  ${CYAN}REST API:${NC}   http://localhost:${PORT}/api/v1"
echo -e "  ${CYAN}WebSocket:${NC}  ws://localhost:${PORT}/ws"
echo ""
echo -e "  ${YELLOW}Trading Markets:${NC}"
echo -e "    - ETH/IUSD (Ethereum)"
echo -e "    - BTC/IUSD (Bitcoin)"
echo -e "    - SOL/IUSD (Solana)"
echo ""
echo -e "  ${YELLOW}Demo Mode:${NC} ${AGENTS} AI agents trading in real-time"
echo ""
echo -e "${BLUE}Press Ctrl+C to stop the server${NC}"
echo ""

# Check if port is in use
if lsof -i:$PORT >/dev/null 2>&1; then
    echo -e "${RED}Error: Port $PORT is already in use${NC}"
    exit 1
fi

# Start the server
exec cargo run -p resonancex-server --release -- --demo --demo-agents "$AGENTS" --port "$PORT"
