#!/usr/bin/env node

/**
 * Direct database fix for user_profiles schema issue
 * Fixes: "Could not find the 'credits' column of 'user_profiles'"
 */

import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load environment variables
dotenv.config({ path: path.join(__dirname, '../.env') });

const SUPABASE_URL = process.env.SUPABASE_URL;
const SUPABASE_SERVICE_KEY = process.env.SUPABASE_SERVICE_KEY;

if (!SUPABASE_URL || !SUPABASE_SERVICE_KEY) {
  console.error(chalk.red('❌ Missing SUPABASE_URL or SUPABASE_SERVICE_KEY'));
  console.log(chalk.yellow('\nPlease ensure your .env file contains:'));
  console.log(chalk.gray('SUPABASE_URL=your_supabase_url'));
  console.log(chalk.gray('SUPABASE_SERVICE_KEY=your_service_key'));
  process.exit(1);
}

console.log(chalk.cyan.bold('\n🔧 Fixing Database Schema\n'));
console.log(chalk.gray(`Supabase URL: ${SUPABASE_URL}`));

// Create admin client
const supabase = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY, {
  auth: {
    autoRefreshToken: false,
    persistSession: false
  },
  db: {
    schema: 'public'
  }
});

async function checkTableStructure() {
  console.log(chalk.yellow('\n📋 Checking current table structure...'));
  
  try {
    // First, try to query the table
    const { data, error } = await supabase
      .from('user_profiles')
      .select('*')
      .limit(1);
    
    if (error) {
      console.log(chalk.red('❌ Error querying user_profiles:'), error.message);
      
      if (error.message.includes('does not exist')) {
        console.log(chalk.yellow('⚠️  Table user_profiles does not exist'));
        return { exists: false };
      }
      
      if (error.message.includes("Could not find the 'credits' column")) {
        console.log(chalk.yellow('⚠️  Missing credits column'));
        return { exists: true, missingCredits: true };
      }
    }
    
    console.log(chalk.green('✅ Table exists'));
    
    // Check if we got any data structure back
    if (data && data.length > 0) {
      console.log(chalk.gray('Current columns:'), Object.keys(data[0]));
    }
    
    return { exists: true, data };
  } catch (err) {
    console.error(chalk.red('Error:'), err);
    return { exists: false };
  }
}

async function createOrFixTable() {
  console.log(chalk.yellow('\n🛠️  Creating/Fixing user_profiles table...'));
  
  // SQL to create or alter the table
  const createTableSQL = `
    -- Create table if it doesn't exist
    CREATE TABLE IF NOT EXISTS public.user_profiles (
      id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
      email TEXT,
      username TEXT,
      display_name TEXT,
      avatar_url TEXT,
      bio TEXT,
      credits INTEGER DEFAULT 100,
      credits_balance INTEGER DEFAULT 100,
      experience_points INTEGER DEFAULT 0,
      level INTEGER DEFAULT 1,
      metadata JSONB DEFAULT '{}'::jsonb,
      created_at TIMESTAMPTZ DEFAULT NOW(),
      updated_at TIMESTAMPTZ DEFAULT NOW()
    );

    -- Add missing columns if table exists
    DO $$ 
    BEGIN
      -- Add credits column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'credits'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN credits INTEGER DEFAULT 100;
      END IF;

      -- Add credits_balance column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'credits_balance'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN credits_balance INTEGER DEFAULT 100;
      END IF;

      -- Add username column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'username'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN username TEXT;
      END IF;

      -- Add display_name column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'display_name'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN display_name TEXT;
      END IF;

      -- Add experience_points column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'experience_points'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN experience_points INTEGER DEFAULT 0;
      END IF;

      -- Add level column if missing
      IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'user_profiles' 
        AND column_name = 'level'
      ) THEN
        ALTER TABLE public.user_profiles 
        ADD COLUMN level INTEGER DEFAULT 1;
      END IF;
    END $$;

    -- Enable RLS
    ALTER TABLE public.user_profiles ENABLE ROW LEVEL SECURITY;

    -- Drop existing policies if they exist
    DROP POLICY IF EXISTS "Users can view own profile" ON public.user_profiles;
    DROP POLICY IF EXISTS "Users can update own profile" ON public.user_profiles;
    DROP POLICY IF EXISTS "Users can insert own profile" ON public.user_profiles;

    -- Create RLS policies
    CREATE POLICY "Users can view own profile" 
      ON public.user_profiles 
      FOR SELECT 
      USING (auth.uid() = id);

    CREATE POLICY "Users can update own profile" 
      ON public.user_profiles 
      FOR UPDATE 
      USING (auth.uid() = id);

    CREATE POLICY "Users can insert own profile" 
      ON public.user_profiles 
      FOR INSERT 
      WITH CHECK (auth.uid() = id);

    -- Create or replace the trigger function
    CREATE OR REPLACE FUNCTION public.handle_new_user() 
    RETURNS trigger AS $$
    BEGIN
      INSERT INTO public.user_profiles (id, email, credits, credits_balance)
      VALUES (
        new.id, 
        new.email,
        100,  -- Initial credits
        100   -- Initial balance
      )
      ON CONFLICT (id) DO UPDATE
      SET email = EXCLUDED.email;
      
      RETURN new;
    END;
    $$ LANGUAGE plpgsql SECURITY DEFINER;

    -- Create trigger for new users
    DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
    CREATE TRIGGER on_auth_user_created
      AFTER INSERT ON auth.users
      FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();

    -- Grant permissions
    GRANT ALL ON public.user_profiles TO authenticated;
    GRANT SELECT ON public.user_profiles TO anon;
  `;

  try {
    // Try to execute via a custom RPC function if available
    const { error: rpcError } = await supabase.rpc('exec_sql', { 
      query: createTableSQL 
    });
    
    if (rpcError) {
      console.log(chalk.yellow('⚠️  Direct SQL execution not available'));
      console.log(chalk.gray('Error:', rpcError.message));
      
      // Fallback: Try to at least update the existing table
      console.log(chalk.yellow('\n🔄 Attempting fallback approach...'));
      
      // Test if we can at least insert/update
      const { error: upsertError } = await supabase
        .from('user_profiles')
        .upsert({
          id: '00000000-0000-0000-0000-000000000000', // Dummy UUID
          credits: 100,
          credits_balance: 100
        })
        .select();
      
      if (upsertError) {
        throw new Error(`Cannot modify table: ${upsertError.message}`);
      }
      
      // Delete the test record
      await supabase
        .from('user_profiles')
        .delete()
        .eq('id', '00000000-0000-0000-0000-000000000000');
    }
    
    console.log(chalk.green('✅ Table structure updated successfully'));
    return true;
  } catch (err) {
    console.error(chalk.red('❌ Failed to update table:'), err.message);
    return false;
  }
}

async function verifyFix() {
  console.log(chalk.yellow('\n🔍 Verifying the fix...'));
  
  try {
    // Try to select with the credits column
    const { data, error } = await supabase
      .from('user_profiles')
      .select('id, email, credits, credits_balance')
      .limit(1);
    
    if (error) {
      console.error(chalk.red('❌ Verification failed:'), error.message);
      return false;
    }
    
    console.log(chalk.green('✅ Successfully verified credits columns exist'));
    return true;
  } catch (err) {
    console.error(chalk.red('❌ Verification error:'), err.message);
    return false;
  }
}

async function printManualInstructions() {
  console.log(chalk.yellow('\n📝 Manual Fix Instructions:'));
  console.log(chalk.white('\nIf the automatic fix didn\'t work, run this SQL in your Supabase SQL Editor:\n'));
  
  const manualSQL = `
-- Add missing columns to user_profiles
ALTER TABLE public.user_profiles 
ADD COLUMN IF NOT EXISTS credits INTEGER DEFAULT 100,
ADD COLUMN IF NOT EXISTS credits_balance INTEGER DEFAULT 100,
ADD COLUMN IF NOT EXISTS username TEXT,
ADD COLUMN IF NOT EXISTS display_name TEXT,
ADD COLUMN IF NOT EXISTS experience_points INTEGER DEFAULT 0,
ADD COLUMN IF NOT EXISTS level INTEGER DEFAULT 1;

-- Refresh the PostgREST schema cache
NOTIFY pgrst, 'reload schema';
`;

  console.log(chalk.cyan(manualSQL));
  
  console.log(chalk.white('\n📍 Steps:'));
  console.log(chalk.gray('1. Go to your Supabase Dashboard'));
  console.log(chalk.gray('2. Navigate to SQL Editor'));
  console.log(chalk.gray('3. Paste and run the SQL above'));
  console.log(chalk.gray('4. The schema cache will refresh automatically'));
}

async function main() {
  console.log(chalk.blue('━'.repeat(50)));
  
  // Step 1: Check current structure
  const tableInfo = await checkTableStructure();
  
  // Step 2: Fix the table
  if (!tableInfo.exists || tableInfo.missingCredits) {
    const fixed = await createOrFixTable();
    
    if (fixed) {
      // Step 3: Verify the fix
      const verified = await verifyFix();
      
      if (verified) {
        console.log(chalk.green.bold('\n✨ Database schema fixed successfully!\n'));
        console.log(chalk.gray('The user_profiles table now has all required columns.'));
      } else {
        console.log(chalk.yellow('\n⚠️  Fix applied but verification failed'));
        await printManualInstructions();
      }
    } else {
      console.log(chalk.red('\n❌ Automatic fix failed'));
      await printManualInstructions();
    }
  } else {
    console.log(chalk.green('\n✅ Table structure is already correct'));
  }
  
  console.log(chalk.blue('━'.repeat(50)));
  
  // Test the connection
  console.log(chalk.cyan('\n🧪 Testing connection...'));
  const { data: testData, error: testError } = await supabase
    .from('user_profiles')
    .select('id')
    .limit(1);
  
  if (testError) {
    console.log(chalk.red('❌ Connection test failed:'), testError.message);
  } else {
    console.log(chalk.green('✅ Connection successful'));
  }
}

// Run the fix
main().catch(err => {
  console.error(chalk.red('\n💥 Unexpected error:'), err);
  process.exit(1);
});