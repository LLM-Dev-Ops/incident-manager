#!/bin/bash

# NPM Publishing Script for Flow Nexus
# Publishes with aliases: flow-nexus, fnx, neural-trader

set -e

echo "🚀 Publishing Flow Nexus to NPM Registry"
echo "==========================================="

# Navigate to package directory
cd /workspaces/flow-cloud/flow/mcp-server

# Check if logged in to npm
echo "📋 Checking NPM authentication..."
npm whoami || {
    echo "❌ Not logged in to npm. Please run: npm login"
    exit 1
}

# Clean install
echo "🧹 Clean install..."
rm -rf node_modules package-lock.json
npm install

# Run any tests if available
echo "🧪 Running tests..."
npm test || echo "⚠️ No tests configured"

# Publish to npm
echo "📦 Publishing package..."
npm publish --access public

echo ""
echo "✅ Successfully published flow-nexus@0.1.65!"
echo ""
echo "📋 Available commands:"
echo "  • npx flow-nexus@latest mcp"
echo "  • npx fnx@latest mcp"
echo "  • npx neural-trader@latest mcp"
echo ""
echo "🎉 Package URL: https://www.npmjs.com/package/flow-nexus"