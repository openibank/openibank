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

    # Clone or update repo
    if [ -d "${INSTALL_DIR}/src" ]; then
        echo -e "${BLUE}Updating existing installation...${NC}"
        cd "${INSTALL_DIR}/src"
        git pull origin main 2>/dev/null || true
    else
        echo -e "${BLUE}Cloning OpeniBank repository...${NC}"
        git clone https://github.com/openibank/openibank.git "${INSTALL_DIR}/src" 2>/dev/null || {
            echo -e "${YELLOW}Git clone failed. Checking for local source...${NC}"
            if [ -f "Cargo.toml" ] && grep -q "openibank" Cargo.toml 2>/dev/null; then
                INSTALL_DIR="$(pwd)"
                echo -e "${GREEN}✓ Using local source directory${NC}"
            else
                echo -e "${RED}Could not find OpeniBank source.${NC}"
                exit 1
            fi
        }
    fi
    
    cd "${INSTALL_DIR}/src" 2>/dev/null || cd "${INSTALL_DIR}"

    # Build
    echo -e "${BLUE}Building OpeniBank CLI...${NC}"
    cargo build --release -p openibank-cli

    # Copy binaries
    cp target/release/openibank "${BIN_DIR}/" 2>/dev/null || true

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
echo "  # Run the viral demo"
echo -e "  ${MAGENTA}openibank demo full${NC}"
echo ""
echo "  # Start the web playground"
echo -e "  ${MAGENTA}openibank-playground${NC}  # Then open http://localhost:8080"
echo ""
echo -e "${YELLOW}AI agents need banks too. This is how they'll pay each other.${NC}"
echo ""
