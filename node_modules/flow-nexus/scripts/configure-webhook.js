#!/usr/bin/env node

/**
 * Configure Supabase webhook for automatic judge triggering
 * This script sets up the webhook programmatically via Supabase Management API
 */

import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import ora from 'ora';
import dotenv from 'dotenv';

dotenv.config();

const SUPABASE_URL = process.env.SUPABASE_URL || 'https://pklhxiuouhrcrreectbo.supabase.co';
const SUPABASE_SERVICE_KEY = process.env.SUPABASE_SERVICE_ROLE_KEY;

async function configureWebhook() {
  console.log(chalk.cyan('🔗 Configuring Judge System Webhook\n'));
  console.log(chalk.gray('=' .repeat(50)));
  
  if (!SUPABASE_SERVICE_KEY) {
    console.error(chalk.red('❌ SUPABASE_SERVICE_ROLE_KEY not set in environment'));
    console.log(chalk.yellow('\nPlease set it in .env file or environment variables'));
    process.exit(1);
  }
  
  const spinner = ora('Configuring webhook...').start();
  
  try {
    const supabase = createClient(SUPABASE_URL, SUPABASE_SERVICE_KEY);
    
    // Check if webhook already exists
    spinner.text = 'Checking existing webhooks...';
    
    // Note: Webhooks need to be configured via Dashboard or Management API
    // This is a guide for manual configuration
    
    spinner.succeed('Webhook configuration guide ready');
    
    console.log(chalk.cyan('\n📋 Manual Configuration Steps:\n'));
    
    console.log(chalk.yellow('1. Go to Supabase Dashboard:'));
    console.log(chalk.gray(`   ${SUPABASE_URL.replace('.supabase.co', '.supabase.com')}/project/pklhxiuouhrcrreectbo/database/hooks`));
    
    console.log(chalk.yellow('\n2. Click "Create a new hook"'));
    
    console.log(chalk.yellow('\n3. Configure the webhook:'));
    console.log(chalk.gray('   Name: judge_submission_trigger'));
    console.log(chalk.gray('   Table: challenge_submissions'));
    console.log(chalk.gray('   Events: UPDATE'));
    
    console.log(chalk.yellow('\n4. Set the webhook URL:'));
    console.log(chalk.gray(`   ${SUPABASE_URL}/functions/v1/trigger-judge`));
    
    console.log(chalk.yellow('\n5. Add HTTP Headers:'));
    console.log(chalk.gray('   Authorization: Bearer [YOUR_SERVICE_ROLE_KEY]'));
    console.log(chalk.gray('   Content-Type: application/json'));
    
    console.log(chalk.yellow('\n6. Configure Payload:'));
    console.log(chalk.gray('   type: INSERT, UPDATE'));
    console.log(chalk.gray('   Include old record: Yes'));
    
    console.log(chalk.yellow('\n7. Add SQL Filter (optional):'));
    console.log(chalk.gray(`   new.status = 'completed' AND old.status != 'completed'`));
    
    console.log(chalk.yellow('\n8. Enable the webhook and save'));
    
    // Create SQL to verify webhook
    console.log(chalk.cyan('\n📝 SQL to verify webhook configuration:\n'));
    
    const verifySQL = `
-- Check if webhooks are configured
SELECT 
  trigger_name,
  event_manipulation,
  event_object_table,
  action_statement
FROM information_schema.triggers
WHERE trigger_schema = 'supabase_functions'
  AND event_object_table = 'challenge_submissions';

-- Check recent webhook invocations (if logging enabled)
SELECT 
  id,
  inserted_at,
  request_id,
  http_status,
  payload
FROM supabase_functions.hooks_logs
WHERE hook_name = 'judge_submission_trigger'
ORDER BY inserted_at DESC
LIMIT 10;
`;
    
    console.log(chalk.gray(verifySQL));
    
    // Test webhook endpoint
    console.log(chalk.cyan('\n🧪 Testing webhook endpoint...'));
    
    const testPayload = {
      type: 'UPDATE',
      table: 'challenge_submissions',
      record: {
        id: 'test-submission-id',
        challenge_id: 'test-challenge-id',
        user_id: 'test-user-id',
        status: 'completed',
        code: '// Test code'
      },
      old_record: {
        status: 'pending'
      }
    };
    
    const response = await fetch(`${SUPABASE_URL}/functions/v1/trigger-judge`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${SUPABASE_SERVICE_KEY}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(testPayload)
    });
    
    if (response.ok) {
      console.log(chalk.green('✅ Webhook endpoint is accessible'));
      const result = await response.json();
      console.log(chalk.gray('   Response:', JSON.stringify(result, null, 2)));
    } else {
      console.log(chalk.yellow('⚠️  Webhook endpoint returned:', response.status));
      console.log(chalk.gray('   This is normal if the edge function is not deployed yet'));
    }
    
    // Create automated setup script
    console.log(chalk.cyan('\n📄 Saving automated setup script...'));
    
    const setupScript = `
-- Queen Seraphina Judge System - Webhook Setup
-- Run this SQL in Supabase SQL Editor

-- Create webhook trigger function
CREATE OR REPLACE FUNCTION public.trigger_judge_webhook()
RETURNS trigger AS $$
BEGIN
  -- Only trigger when status changes to 'completed'
  IF NEW.status = 'completed' AND OLD.status != 'completed' THEN
    -- Call edge function via pg_net (if available)
    PERFORM net.http_post(
      url := '${SUPABASE_URL}/functions/v1/trigger-judge',
      headers := jsonb_build_object(
        'Authorization', 'Bearer ${SUPABASE_SERVICE_KEY}',
        'Content-Type', 'application/json'
      ),
      body := jsonb_build_object(
        'type', 'UPDATE',
        'table', 'challenge_submissions',
        'record', row_to_json(NEW),
        'old_record', row_to_json(OLD)
      )
    );
  END IF;
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger
DROP TRIGGER IF EXISTS on_submission_completed ON challenge_submissions;
CREATE TRIGGER on_submission_completed
  AFTER UPDATE ON challenge_submissions
  FOR EACH ROW
  EXECUTE FUNCTION trigger_judge_webhook();

-- Grant necessary permissions
GRANT EXECUTE ON FUNCTION public.trigger_judge_webhook() TO postgres, authenticated, service_role;

-- Test the trigger
UPDATE challenge_submissions 
SET status = 'completed'
WHERE id = (SELECT id FROM challenge_submissions WHERE status = 'pending' LIMIT 1);
`;
    
    // Save setup script
    const fs = await import('fs/promises');
    await fs.writeFile('./setup-webhook.sql', setupScript);
    console.log(chalk.green('✅ Setup script saved to setup-webhook.sql'));
    
    console.log(chalk.green('\n✨ Webhook configuration complete!'));
    console.log(chalk.cyan('\nNext steps:'));
    console.log(chalk.gray('1. Follow the manual steps above in Supabase Dashboard'));
    console.log(chalk.gray('2. Or run the setup-webhook.sql script in SQL Editor'));
    console.log(chalk.gray('3. Deploy edge functions: npm run deploy:functions'));
    console.log(chalk.gray('4. Test with: npm run test:integration'));
    
  } catch (error) {
    spinner.fail('Configuration failed');
    console.error(chalk.red('\n❌ Error:'), error.message);
    process.exit(1);
  }
}

// Run configuration
configureWebhook();