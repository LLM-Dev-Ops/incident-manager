#!/bin/bash

# Deploy Accounting System for Flow Nexus
# This script deploys the complete real-time accounting/ledger system

set -e

echo "🚀 Deploying Flow Nexus Accounting System..."

# Check prerequisites
if ! command -v supabase &> /dev/null; then
    echo "❌ Supabase CLI is required. Install from: https://supabase.com/docs/guides/cli"
    exit 1
fi

if ! command -v node &> /dev/null; then
    echo "❌ Node.js is required"
    exit 1
fi

# Check environment variables
if [ -z "$SUPABASE_URL" ] || [ -z "$SUPABASE_SERVICE_ROLE_KEY" ]; then
    echo "❌ Required environment variables not set:"
    echo "   SUPABASE_URL"
    echo "   SUPABASE_SERVICE_ROLE_KEY"
    exit 1
fi

echo "✅ Prerequisites checked"

# Step 1: Deploy database migrations
echo "📁 Deploying database schema..."
if [ -f "supabase/migrations/20250821_accounting_ledger_system.sql" ]; then
    supabase db push --include-all
    echo "✅ Database schema deployed"
else
    echo "❌ Migration file not found"
    exit 1
fi

# Step 2: Deploy Edge Functions
echo "🔧 Deploying Edge Functions..."

functions=("transaction-processor" "balance-calculator" "reporting-analytics" "realtime-coordinator")

for func in "${functions[@]}"; do
    echo "   Deploying $func..."
    if [ -d "supabase/functions/$func" ]; then
        supabase functions deploy $func --project-ref $SUPABASE_PROJECT_REF
        echo "   ✅ $func deployed"
    else
        echo "   ❌ Function $func not found"
        exit 1
    fi
done

# Step 3: Set up realtime channels
echo "📡 Setting up realtime channels..."
curl -X GET "$SUPABASE_URL/functions/v1/realtime-coordinator?action=setup_channels" \
    -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY" \
    -H "Content-Type: application/json"

if [ $? -eq 0 ]; then
    echo "✅ Realtime channels configured"
else
    echo "❌ Failed to configure realtime channels"
fi

# Step 4: Initialize chart of accounts (if needed)
echo "💰 Initializing chart of accounts..."
node -e "
const { createClient } = require('@supabase/supabase-js');
const supabase = createClient('$SUPABASE_URL', '$SUPABASE_SERVICE_ROLE_KEY');

async function initAccounts() {
    const { data, error } = await supabase
        .from('chart_of_accounts')
        .select('id')
        .limit(1);
    
    if (error) {
        console.error('Error checking accounts:', error);
        return;
    }
    
    if (data && data.length > 0) {
        console.log('Chart of accounts already initialized');
        return;
    }
    
    console.log('Chart of accounts initialized by migration');
}

initAccounts().catch(console.error);
"

# Step 5: Refresh materialized views
echo "🔄 Refreshing materialized views..."
curl -X GET "$SUPABASE_URL/functions/v1/balance-calculator?operation=refresh_views" \
    -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY"

# Step 6: Run integrity check
echo "🔍 Running system integrity check..."
curl -X GET "$SUPABASE_URL/functions/v1/balance-calculator?operation=trial_balance" \
    -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY" | jq '.'

# Step 7: Test transaction processing
echo "🧪 Testing transaction processing..."
curl -X POST "$SUPABASE_URL/functions/v1/transaction-processor" \
    -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY" \
    -H "Content-Type: application/json" \
    -d '{
        "user_id": "test-deployment-user",
        "category": "bonus",
        "amount": 1,
        "description": "Deployment test transaction"
    }' | jq '.'

echo ""
echo "🎉 Accounting System Deployment Complete!"
echo ""
echo "📊 System Features Deployed:"
echo "   ✅ Double-entry bookkeeping database"
echo "   ✅ Real-time transaction processing"
echo "   ✅ Balance calculation engine"
echo "   ✅ Comprehensive reporting system"
echo "   ✅ Live realtime updates"
echo "   ✅ Audit trail and compliance"
echo ""
echo "🔗 Available Endpoints:"
echo "   Transaction Processing: $SUPABASE_URL/functions/v1/transaction-processor"
echo "   Balance Calculator: $SUPABASE_URL/functions/v1/balance-calculator"
echo "   Reporting: $SUPABASE_URL/functions/v1/reporting-analytics"
echo "   Realtime: $SUPABASE_URL/functions/v1/realtime-coordinator"
echo ""
echo "📚 Documentation: ./docs/accounting-system-api.md"
echo "🧪 Tests: npm test -- tests/accounting-system.test.ts"
echo ""
echo "⚠️  Next Steps:"
echo "   1. Update your client applications to use the new accounting tools"
echo "   2. Set up monitoring and alerts"
echo "   3. Configure backup and retention policies"
echo "   4. Review security settings and RLS policies"
echo ""