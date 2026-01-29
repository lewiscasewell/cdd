#!/bin/bash
set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Get version from Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
TAG="v$VERSION"

echo -e "${YELLOW}Preparing release ${TAG}${NC}"
echo ""

# Check for uncommitted changes
if [[ -n $(git status --porcelain) ]]; then
    echo -e "${RED}Error: You have uncommitted changes${NC}"
    git status --short
    exit 1
fi

# Check if tag already exists
if git rev-parse "$TAG" >/dev/null 2>&1; then
    echo -e "${RED}Error: Tag ${TAG} already exists${NC}"
    echo "Update the version in Cargo.toml first"
    exit 1
fi

# Run tests
echo -e "${YELLOW}Running tests...${NC}"
cargo test --quiet
echo -e "${GREEN}Tests passed${NC}"
echo ""

# Show what will be released
echo -e "${YELLOW}Commits since last tag:${NC}"
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
if [[ -n "$LAST_TAG" ]]; then
    git log ${LAST_TAG}..HEAD --oneline
else
    echo "(first release)"
fi
echo ""

# Confirm
echo -e "${YELLOW}Ready to release ${TAG}${NC}"
read -p "Continue? (y/N) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted"
    exit 1
fi

# Create and push tag
echo -e "${YELLOW}Creating tag ${TAG}...${NC}"
git tag -a "$TAG" -m "Release $TAG"

echo -e "${YELLOW}Pushing to origin...${NC}"
git push origin main --tags

echo ""
echo -e "${GREEN}Done! Release ${TAG} triggered.${NC}"
echo "Watch the release at: https://github.com/lewiscasewell/cdd/actions"
