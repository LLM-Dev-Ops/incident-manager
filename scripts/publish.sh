#!/bin/bash
#
# Publish llm-incident-manager to crates.io
#
# Usage:
#   ./scripts/publish.sh [--dry-run] [--token YOUR_TOKEN]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse arguments
DRY_RUN=false
TOKEN=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --token)
            TOKEN="$2"
            shift 2
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            exit 1
            ;;
    esac
done

echo -e "${GREEN}=== Publishing llm-incident-manager to crates.io ===${NC}\n"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Rust not found. Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo -e "${GREEN}Rust installed successfully!${NC}\n"
fi

# Display Rust version
echo -e "${GREEN}Rust version:${NC}"
rustc --version
cargo --version
echo ""

# Set token if provided
if [ -n "$TOKEN" ]; then
    echo -e "${GREEN}Using provided CARGO_REGISTRY_TOKEN${NC}"
    export CARGO_REGISTRY_TOKEN="$TOKEN"
elif [ -n "$CARGO_REGISTRY_TOKEN" ]; then
    echo -e "${GREEN}Using CARGO_REGISTRY_TOKEN from environment${NC}"
else
    echo -e "${YELLOW}No token found. You'll need to run 'cargo login' first.${NC}"
    echo -e "${YELLOW}Get your token from: https://crates.io/me${NC}\n"
fi

# Run tests
echo -e "${GREEN}Running tests...${NC}"
cargo test --all-features || {
    echo -e "${RED}Tests failed! Aborting publish.${NC}"
    exit 1
}
echo -e "${GREEN}Tests passed!${NC}\n"

# Run dry-run
echo -e "${GREEN}Running dry-run...${NC}"
cargo publish --dry-run || {
    echo -e "${RED}Dry-run failed! Aborting publish.${NC}"
    exit 1
}
echo -e "${GREEN}Dry-run succeeded!${NC}\n"

if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}Dry-run mode - not publishing to crates.io${NC}"
    exit 0
fi

# Confirm publication
echo -e "${YELLOW}Ready to publish llm-incident-manager v1.0.0 to crates.io${NC}"
read -p "Continue? (y/N): " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo -e "${YELLOW}Publish cancelled${NC}"
    exit 0
fi

# Publish
echo -e "${GREEN}Publishing to crates.io...${NC}"
cargo publish || {
    echo -e "${RED}Publish failed!${NC}"
    exit 1
}

echo -e "\n${GREEN}=== Successfully published to crates.io! ===${NC}"
echo -e "${GREEN}View at: https://crates.io/crates/llm-incident-manager${NC}"
echo -e "\n${GREEN}Users can install with:${NC}"
echo -e "  cargo install llm-incident-manager"
echo -e "  cargo install llm-incident-manager --bin llm-im-cli"
