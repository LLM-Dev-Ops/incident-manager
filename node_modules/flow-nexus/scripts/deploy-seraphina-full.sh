#!/bin/bash

echo "🚀 Deploying Queen Seraphina Edge Function with Database Setup..."

# Load environment variables
source .env

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check for required environment variables
if [ -z "$SUPABASE_URL" ] || [ -z "$SUPABASE_ANON_KEY" ]; then
    echo -e "${RED}❌ Missing SUPABASE_URL or SUPABASE_ANON_KEY in .env${NC}"
    exit 1
fi

echo -e "${YELLOW}📋 Step 1: Creating database tables for Seraphina...${NC}"

# Create SQL file for database setup
cat > /tmp/seraphina_setup.sql << 'EOF'
-- Create seraphina_interactions table for chat history
CREATE TABLE IF NOT EXISTS public.seraphina_interactions (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID REFERENCES auth.users(id) ON DELETE CASCADE,
    messages JSONB NOT NULL,
    response TEXT,
    credits_used INTEGER DEFAULT 1,
    tools_used BOOLEAN DEFAULT false,
    deployment_id TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    metadata JSONB DEFAULT '{}'::jsonb
);

-- Create index for faster queries
CREATE INDEX IF NOT EXISTS idx_seraphina_interactions_user_id 
ON public.seraphina_interactions(user_id);

CREATE INDEX IF NOT EXISTS idx_seraphina_interactions_created_at 
ON public.seraphina_interactions(created_at DESC);

-- Enable RLS
ALTER TABLE public.seraphina_interactions ENABLE ROW LEVEL SECURITY;

-- Create RLS policies for user isolation
CREATE POLICY "Users can view own interactions" 
ON public.seraphina_interactions 
FOR SELECT 
USING (auth.uid() = user_id);

CREATE POLICY "Users can insert own interactions" 
ON public.seraphina_interactions 
FOR INSERT 
WITH CHECK (auth.uid() = user_id);

-- Create secure vault table for API keys if not exists
CREATE TABLE IF NOT EXISTS public.vault (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    key TEXT UNIQUE NOT NULL,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Enable RLS on vault
ALTER TABLE public.vault ENABLE ROW LEVEL SECURITY;

-- Only service role can access vault
CREATE POLICY "Service role only" 
ON public.vault 
FOR ALL 
USING (auth.role() = 'service_role');

-- Create function to obfuscate sensitive data
CREATE OR REPLACE FUNCTION obfuscate_token(token TEXT)
RETURNS TEXT AS $$
BEGIN
    IF token IS NULL OR LENGTH(token) < 10 THEN
        RETURN token;
    END IF;
    RETURN SUBSTRING(token, 1, 6) || '...' || SUBSTRING(token, LENGTH(token) - 3);
END;
$$ LANGUAGE plpgsql;

-- Create function to get user context for Seraphina
CREATE OR REPLACE FUNCTION get_user_context_for_seraphina(p_user_id UUID)
RETURNS JSONB AS $$
DECLARE
    v_context JSONB;
    v_swarm_count INTEGER;
    v_challenge_count INTEGER;
    v_sandbox_count INTEGER;
    v_recent_interactions INTEGER;
BEGIN
    -- Get swarm count
    SELECT COUNT(*) INTO v_swarm_count
    FROM public.swarms
    WHERE user_id = p_user_id AND status = 'active';
    
    -- Get challenge count
    SELECT COUNT(*) INTO v_challenge_count
    FROM public.challenge_submissions
    WHERE user_id = p_user_id AND status = 'completed';
    
    -- Get sandbox count
    SELECT COUNT(*) INTO v_sandbox_count
    FROM public.sandboxes
    WHERE user_id = p_user_id AND status = 'running';
    
    -- Get recent interaction count
    SELECT COUNT(*) INTO v_recent_interactions
    FROM public.seraphina_interactions
    WHERE user_id = p_user_id 
    AND created_at > NOW() - INTERVAL '7 days';
    
    v_context := jsonb_build_object(
        'swarms', v_swarm_count,
        'challenges', v_challenge_count,
        'sandboxes', v_sandbox_count,
        'recent_chats', v_recent_interactions,
        'last_seen', NOW()
    );
    
    RETURN v_context;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Create RPC function for secure credit deduction
CREATE OR REPLACE FUNCTION deduct_credits_for_seraphina(
    p_user_id UUID,
    p_amount INTEGER DEFAULT 1,
    p_interaction_id UUID DEFAULT NULL
)
RETURNS JSONB AS $$
DECLARE
    v_current_balance INTEGER;
    v_new_balance INTEGER;
BEGIN
    -- Get current balance with lock
    SELECT credits_balance INTO v_current_balance
    FROM public.profiles
    WHERE id = p_user_id
    FOR UPDATE;
    
    -- Check sufficient balance
    IF v_current_balance < p_amount THEN
        RETURN jsonb_build_object(
            'success', false,
            'error', 'Insufficient credits',
            'balance', v_current_balance,
            'required', p_amount
        );
    END IF;
    
    -- Deduct credits
    v_new_balance := v_current_balance - p_amount;
    
    UPDATE public.profiles
    SET credits_balance = v_new_balance,
        updated_at = NOW()
    WHERE id = p_user_id;
    
    -- Log transaction
    INSERT INTO public.ruv_transactions (
        user_id,
        amount,
        transaction_type,
        description,
        metadata,
        balance_after
    ) VALUES (
        p_user_id,
        -p_amount,
        'debit',
        'Queen Seraphina Chat',
        jsonb_build_object('interaction_id', p_interaction_id),
        v_new_balance
    );
    
    RETURN jsonb_build_object(
        'success', true,
        'previous_balance', v_current_balance,
        'new_balance', v_new_balance,
        'amount_deducted', p_amount
    );
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- Grant execute permissions
GRANT EXECUTE ON FUNCTION get_user_context_for_seraphina TO authenticated;
GRANT EXECUTE ON FUNCTION deduct_credits_for_seraphina TO authenticated;
GRANT EXECUTE ON FUNCTION obfuscate_token TO authenticated;

COMMENT ON TABLE public.seraphina_interactions IS 'Stores chat history with Queen Seraphina';
COMMENT ON FUNCTION get_user_context_for_seraphina IS 'Gets user context for personalized Seraphina responses';
COMMENT ON FUNCTION deduct_credits_for_seraphina IS 'Atomic credit deduction for Seraphina chat';
EOF

echo -e "${YELLOW}📦 Step 2: Applying database changes...${NC}"

# Execute SQL using psql with connection string
if command -v psql &> /dev/null; then
    # Extract database connection details from SUPABASE_URL
    DB_HOST=$(echo $SUPABASE_URL | sed 's/https:\/\///' | sed 's/\.supabase\.co//')
    psql "postgresql://postgres.${DB_HOST}:6543/postgres?sslmode=require" -f /tmp/seraphina_setup.sql
else
    echo -e "${YELLOW}⚠️  psql not found. Please run the SQL manually in Supabase SQL editor${NC}"
    echo "SQL saved to: /tmp/seraphina_setup.sql"
fi

echo -e "${YELLOW}🔧 Step 3: Preparing edge function...${NC}"

# Ensure edge function directory exists
FUNCTION_DIR="supabase/functions/seraphina-chat"
mkdir -p $FUNCTION_DIR

# Update edge function to use .env values
cat > $FUNCTION_DIR/config.ts << 'EOF'
// Configuration for Seraphina Edge Function
export const config = {
  // These will be set as secrets in Supabase
  anthropicKey: Deno.env.get('ANTHROPIC_API_KEY'),
  supabaseUrl: Deno.env.get('SUPABASE_URL'),
  supabaseServiceKey: Deno.env.get('SUPABASE_SERVICE_ROLE_KEY'),
  
  // Chat configuration
  creditCost: 1,
  maxContextMessages: 10,
  model: 'claude-3-opus-20240229',
  maxTokens: 4096,
  temperature: 0.8,
  
  // Security
  obfuscateTokens: true,
  maxGrantAmount: 100,
  
  // Rate limiting
  maxMessagesPerMinute: 10,
  maxMessagesPerHour: 100
};
EOF

echo -e "${YELLOW}🚀 Step 4: Deploying edge function to Supabase...${NC}"

# Check if supabase CLI is installed
if ! command -v supabase &> /dev/null; then
    echo -e "${RED}❌ Supabase CLI not found. Installing...${NC}"
    npm install -g supabase
fi

# Initialize supabase if needed
if [ ! -f "supabase/config.toml" ]; then
    echo -e "${YELLOW}Initializing Supabase project...${NC}"
    supabase init
fi

# Link to project using project ref from URL
PROJECT_REF=$(echo $SUPABASE_URL | sed 's/https:\/\///' | sed 's/\.supabase\.co//')
echo -e "${YELLOW}Linking to project: $PROJECT_REF${NC}"
supabase link --project-ref $PROJECT_REF

# Deploy the edge function
echo -e "${YELLOW}Deploying seraphina-chat function...${NC}"
supabase functions deploy seraphina-chat --no-verify-jwt

# Set secrets for the edge function
echo -e "${YELLOW}🔐 Step 5: Setting edge function secrets...${NC}"

# Check if ANTHROPIC_API_KEY exists in environment
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo -e "${YELLOW}Enter your Anthropic API key (or press Enter to skip):${NC}"
    read -s ANTHROPIC_API_KEY
fi

if [ ! -z "$ANTHROPIC_API_KEY" ]; then
    supabase secrets set ANTHROPIC_API_KEY=$ANTHROPIC_API_KEY
    echo -e "${GREEN}✅ Anthropic API key set${NC}"
else
    echo -e "${YELLOW}⚠️  No Anthropic API key set. Add it later with: supabase secrets set ANTHROPIC_API_KEY=your-key${NC}"
fi

# Get service role key
if [ -z "$SUPABASE_SERVICE_ROLE_KEY" ]; then
    echo -e "${YELLOW}Enter your Supabase service role key (or press Enter to skip):${NC}"
    read -s SUPABASE_SERVICE_ROLE_KEY
fi

if [ ! -z "$SUPABASE_SERVICE_ROLE_KEY" ]; then
    supabase secrets set SUPABASE_SERVICE_ROLE_KEY=$SUPABASE_SERVICE_ROLE_KEY
    echo -e "${GREEN}✅ Service role key set${NC}"
fi

echo -e "${GREEN}✨ Queen Seraphina deployment complete!${NC}"
echo ""
echo -e "${GREEN}Test with:${NC}"
echo "  npx flow-nexus seraphina"
echo "  npx flow-nexus chat \"Hello Queen Seraphina\""
echo ""
echo -e "${GREEN}Or via MCP:${NC}"
echo "  mcp.seraphina_chat({ message: \"Guide me, Queen Seraphina\" })"
echo ""
echo -e "${YELLOW}📝 Notes:${NC}"
echo "  - Cost: 1 rUv credit per message"
echo "  - Chat history stored securely in database"
echo "  - User data isolated by RLS policies"
echo "  - Tokens automatically obfuscated in logs"
echo ""
echo -e "${GREEN}👑 The Queen awaits your audience!${NC}"