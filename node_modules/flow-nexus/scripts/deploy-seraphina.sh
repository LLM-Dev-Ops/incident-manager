#!/bin/bash

echo "🚀 Deploying Queen Seraphina Edge Function..."

# Deploy the edge function
npx supabase functions deploy seraphina-chat \
  --project-ref qkfbgtzuylbtabwffsti \
  --no-verify-jwt

echo "✅ Queen Seraphina is ready to grant audiences!"
echo ""
echo "Test with:"
echo "  npx flow-nexus seraphina"
echo "  npx flow-nexus chat \"How do I get started?\""
echo ""
echo "Or use the MCP tool:"
echo "  mcp.seraphina_chat({ message: \"Hello Queen Seraphina\" })"