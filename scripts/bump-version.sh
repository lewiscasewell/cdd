#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

CURRENT=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

if [[ -z "$1" ]]; then
    echo "Current version: $CURRENT"
    echo ""
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.3.0"
    exit 1
fi

NEW_VERSION=$1

# Validate semver format
if [[ ! "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo -e "${RED}Error: Version must be in format X.Y.Z${NC}"
    exit 1
fi

echo -e "${YELLOW}Bumping version: ${CURRENT} → ${NEW_VERSION}${NC}"

# Update Cargo.toml
sed -i '' "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml

# Verify
NEW_CHECK=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [[ "$NEW_CHECK" != "$NEW_VERSION" ]]; then
    echo -e "${RED}Error: Version update failed${NC}"
    exit 1
fi

echo -e "${GREEN}Updated Cargo.toml to version ${NEW_VERSION}${NC}"
echo ""
echo "Next steps:"
echo "  git add Cargo.toml"
echo "  git commit -m \"Bump version to ${NEW_VERSION}\""
echo "  ./scripts/release.sh"
