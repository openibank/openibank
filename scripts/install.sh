#!/bin/bash
# OpeniBank Installer
# Usage: curl -sSL https://openibank.com/install.sh | bash
#
# This script installs the OpeniBank CLI and optionally sets up Ollama for local LLM support.

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
MAGENTA='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Banner
echo -e "${MAGENTA}"
cat << 'BANNER'
╔═══════════════════════════════════════════════════════════════╗
║                                                               ║
║    🏦 OpeniBank - Banking for AI Agents                       ║
║                                                               ║
║    AI agents need banks too.                                  ║
║    This is how they'll pay each other.                        ║
║                                                               ║
╚═══════════════════════════════════════════════════════════════╝
BANNER
echo -e "${NC}"

# Detect OS
OS="unknown"
case "$(uname -s)" in
    Linux*)     OS="linux";;
    Darwin*)    OS="macos";;
    MINGW*|MSYS*|CYGWIN*) OS="windows";;
esac

# Detect architecture
ARCH="unknown"
case "$(uname -m)" in
    x86_64|amd64)  ARCH="x86_64";;
    aarch64|arm64) ARCH="aarch64";;
esac

echo -e "${CYAN}Detected: ${OS} / ${ARCH}${NC}"
echo ""

# Installation directory
INSTALL_DIR="${HOME}/.openibank"
BIN_DIR="${INSTALL_DIR}/bin"

mkdir -p "$BIN_DIR"

# Check if we should build from source (cargo available) or download binary
if command -v cargo &> /dev/null; then
    echo -e "${GREEN}✓ Rust/Cargo detected. Building from source...${NC}"
    echo ""

    # Clone or update maple (required dependency - must be sibling to openibank)
    if [ -d "${INSTALL_DIR}/maple" ]; then
        echo -e "${BLUE}Updating maple framework...${NC}"
        cd "${INSTALL_DIR}/maple"
        git pull origin main 2>/dev/null || true
    else
        echo -e "${BLUE}Cloning maple framework (required dependency)...${NC}"
        git clone https://github.com/mapleaiorg/maple.git "${INSTALL_DIR}/maple" 2>/dev/null || {
            echo -e "${YELLOW}Could not auto-clone maple. You may need to clone it manually:${NC}"
            echo "  git clone <maple-repo-url> ${INSTALL_DIR}/maple"
        }
    fi

    # Clone or update openibank
    if [ -d "${INSTALL_DIR}/src" ]; then
        echo -e "${BLUE}Updating existing installation...${NC}"
        cd "${INSTALL_DIR}/src"
        git pull origin main 2>/dev/null || true
    else
        echo -e "${BLUE}Cloning OpeniBank repository...${NC}"
        git clone https://github.com/openibank/openibank.git "${INSTALL_DIR}/src" 2>/dev/null || {
            echo -e "${YELLOW}Git clone failed. Checking for local source...${NC}"
            if [ -f "Cargo.toml" ] && grep -q "openibank" Cargo.toml 2>/dev/null; then
                INSTALL_DIR="$(pwd)/.."
                echo -e "${GREEN}✓ Using local source directory${NC}"
            else
                echo -e "${RED}Could not find OpeniBank source.${NC}"
                exit 1
            fi
        }
    fi

    cd "${INSTALL_DIR}/src" 2>/dev/null || cd "${INSTALL_DIR}"

    # Verify maple is in the expected location (sibling ../maple)
    if [ ! -d "../maple/crates" ]; then
        echo -e "${RED}Error: maple framework not found at $(cd .. && pwd)/maple${NC}"
        echo -e "${YELLOW}OpeniBank requires the maple framework as a sibling directory.${NC}"
        echo -e "${YELLOW}Expected layout:${NC}"
        echo "  parent_dir/"
        echo "    ├── maple/        # Maple AI Framework"
        echo "    └── openibank/    # OpeniBank (this project)"
        echo ""
        echo -e "${YELLOW}Clone maple manually and re-run:${NC}"
        echo "  git clone <maple-repo-url> $(cd .. && pwd)/maple"
        exit 1
    fi
    echo -e "${GREEN}✓ maple framework found${NC}"

    # Build
    echo -e "${BLUE}Building OpeniBank CLI...${NC}"
    cargo build --release -p openibank-cli

    # Copy binaries
    cp target/release/openibank "${BIN_DIR}/" 2>/dev/null || true

    # Build the unified server
    echo -e "${BLUE}Building OpeniBank Server (unified binary)...${NC}"
    cargo build --release -p openibank-server 2>/dev/null || true
    cp target/release/openibank-server "${BIN_DIR}/" 2>/dev/null || true

    # Build additional services
    echo -e "${BLUE}Building additional services...${NC}"
    cargo build --release -p openibank-playground 2>/dev/null || true
    cargo build --release -p openibank-mcp 2>/dev/null || true

    # Copy service binaries
    for bin in openibank-playground openibank-mcp openibank-issuer-resonator; do
        cp "target/release/${bin}" "${BIN_DIR}/" 2>/dev/null || true
    done

else
    echo -e "${YELLOW}⚠ Rust not found. Please install Rust first:${NC}"
    echo "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

echo ""
echo -e "${GREEN}✓ OpeniBank CLI installed to ${BIN_DIR}/openibank${NC}"

# Add to PATH
SHELL_NAME=$(basename "$SHELL")
PROFILE_FILE=""

case "$SHELL_NAME" in
    bash)
        if [ -f "${HOME}/.bash_profile" ]; then
            PROFILE_FILE="${HOME}/.bash_profile"
        else
            PROFILE_FILE="${HOME}/.bashrc"
        fi
        ;;
    zsh)
        PROFILE_FILE="${HOME}/.zshrc"
        ;;
esac

if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
    if [ -n "$PROFILE_FILE" ]; then
        echo "" >> "$PROFILE_FILE"
        echo "# OpeniBank" >> "$PROFILE_FILE"
        echo "export PATH=\"\$PATH:${BIN_DIR}\"" >> "$PROFILE_FILE"
        echo -e "${GREEN}✓ Added to PATH in ${PROFILE_FILE}${NC}"
    fi
fi

echo ""
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  ✓ Installation Complete!${NC}"
echo -e "${GREEN}════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${CYAN}Quick Start:${NC}"
echo ""
echo "  # Start the AI Agent Banking Server"
echo -e "  ${MAGENTA}openibank-server${NC}              # http://localhost:8080"
echo ""
echo "  # With Anthropic Claude LLM"
echo -e "  ${MAGENTA}OPENIBANK_LLM_PROVIDER=anthropic ANTHROPIC_API_KEY=sk-... openibank-server${NC}"
echo ""
echo "  # With local Ollama LLM"
echo -e "  ${MAGENTA}OPENIBANK_LLM_PROVIDER=ollama openibank-server${NC}"
echo ""
echo "  # Run the demo"
echo -e "  ${MAGENTA}openibank demo full${NC}"
echo ""
echo "  # Start the web playground"
echo -e "  ${MAGENTA}openibank-playground${NC}          # http://localhost:8080"
echo ""
echo -e "${YELLOW}AI agents need banks too. This is how they'll pay each other.${NC}"
echo -e "${CYAN}https://www.openibank.com${NC}"
echo ""
