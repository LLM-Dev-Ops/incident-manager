#!/bin/bash

# Deploy Queen Seraphina Judge System Edge Functions
# This script deploys the edge functions and configures webhooks

echo "🚀 Deploying Queen Seraphina Judge System"
echo "========================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check for required environment variables
check_env() {
    if [ -z "$SUPABASE_URL" ]; then
        echo -e "${RED}❌ SUPABASE_URL not set${NC}"
        exit 1
    fi
    if [ -z "$SUPABASE_SERVICE_ROLE_KEY" ]; then
        echo -e "${RED}❌ SUPABASE_SERVICE_ROLE_KEY not set${NC}"
        exit 1
    fi
    if [ -z "$ANTHROPIC_API_KEY" ]; then
        echo -e "${YELLOW}⚠️  ANTHROPIC_API_KEY not set (required for production)${NC}"
    fi
    if [ -z "$E2B_API_KEY" ]; then
        echo -e "${YELLOW}⚠️  E2B_API_KEY not set (required for production)${NC}"
    fi
}

# Deploy using Supabase CLI (if available) or via API
deploy_functions() {
    echo -e "${YELLOW}📦 Deploying Edge Functions...${NC}"
    
    # Check if Supabase CLI is available
    if command -v supabase &> /dev/null; then
        echo "Using Supabase CLI..."
        
        # Deploy trigger-judge function
        cd ../supabase/functions
        supabase functions deploy trigger-judge \
            --no-verify-jwt \
            --import-map ../import_map.json
        
        # Deploy judge-challenge function  
        supabase functions deploy judge-challenge \
            --no-verify-jwt \
            --import-map ../import_map.json
            
        echo -e "${GREEN}✅ Functions deployed via CLI${NC}"
    else
        echo -e "${YELLOW}Supabase CLI not found. Use Supabase Dashboard to deploy:${NC}"
        echo "1. Go to https://app.supabase.com/project/pklhxiuouhrcrreectbo/functions"
        echo "2. Click 'New Function'"
        echo "3. Upload the function code from:"
        echo "   - supabase/functions/trigger-judge/index.ts"
        echo "   - supabase/functions/judge-challenge/index.ts"
        echo "4. Set environment variables in function settings"
    fi
}

# Configure database webhook
configure_webhook() {
    echo -e "${YELLOW}🔗 Configuring Database Webhook...${NC}"
    
    cat << EOF

To configure the webhook in Supabase Dashboard:

1. Go to Database → Webhooks
2. Click "Create a new webhook"
3. Configure as follows:

   Name: judge_submission_trigger
   Table: challenge_submissions
   Events: UPDATE
   
   URL: https://pklhxiuouhrcrreectbo.supabase.co/functions/v1/trigger-judge
   
   HTTP Headers:
   - Authorization: Bearer ${SUPABASE_SERVICE_ROLE_KEY}
   - Content-Type: application/json
   
   Payload configuration:
   - type: record
   - old_record: true

4. Add filter (optional):
   new.status = 'completed' AND old.status != 'completed'

5. Click "Create webhook"

EOF
}

# Test the deployment
test_deployment() {
    echo -e "${YELLOW}🧪 Testing Deployment...${NC}"
    
    # Test if functions are accessible
    TRIGGER_URL="https://pklhxiuouhrcrreectbo.supabase.co/functions/v1/trigger-judge"
    JUDGE_URL="https://pklhxiuouhrcrreectbo.supabase.co/functions/v1/judge-challenge"
    
    echo "Testing trigger-judge function..."
    curl -s -o /dev/null -w "%{http_code}" \
        -X OPTIONS \
        "$TRIGGER_URL" | grep -q "200" && \
        echo -e "${GREEN}✅ trigger-judge function is accessible${NC}" || \
        echo -e "${RED}❌ trigger-judge function not accessible${NC}"
    
    echo "Testing judge-challenge function..."
    curl -s -o /dev/null -w "%{http_code}" \
        -X OPTIONS \
        "$JUDGE_URL" | grep -q "200" && \
        echo -e "${GREEN}✅ judge-challenge function is accessible${NC}" || \
        echo -e "${RED}❌ judge-challenge function not accessible${NC}"
}

# Set environment variables for functions
set_env_vars() {
    echo -e "${YELLOW}🔑 Setting Environment Variables...${NC}"
    
    cat << EOF

Set these environment variables in Supabase Dashboard:

1. Go to Project Settings → Edge Functions
2. Add the following secrets:

   ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-"<your-key>"}
   E2B_API_KEY=${E2B_API_KEY:-"<your-key>"}
   FLOW_NEXUS_API_URL=${FLOW_NEXUS_API_URL:-"https://pklhxiuouhrcrreectbo.supabase.co"}
   FLOW_NEXUS_API_KEY=${SUPABASE_SERVICE_ROLE_KEY}
   
3. These will be available to all edge functions

EOF
}

# Main execution
echo "Starting deployment process..."
echo ""

check_env
deploy_functions
configure_webhook
set_env_vars
test_deployment

echo ""
echo -e "${GREEN}🎉 Deployment configuration complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Complete manual steps in Supabase Dashboard if needed"
echo "2. Run the test script: npm run test:judge"
echo "3. Monitor logs in Supabase Dashboard → Logs → Edge Functions"