#!/usr/bin/env node

import fs from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';
import crypto from 'crypto';
import readline from 'readline/promises';
import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import dotenv from 'dotenv';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.join(__dirname, '..');
const ENV_PATH = path.join(ROOT_DIR, '.env');
const ENV_LOCAL_PATH = path.join(ROOT_DIR, '.env.local');

// Load existing environment variables
dotenv.config({ path: ENV_PATH });
dotenv.config({ path: ENV_LOCAL_PATH });

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Supabase configuration (public endpoint for registration)
const SUPABASE_URL = process.env.SUPABASE_URL || 'https://kklkivksddkcazkvfjsw.supabase.co';
const SUPABASE_ANON_KEY = process.env.SUPABASE_ANON_KEY || 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImtrbGtpdmtzZGRrY2F6a3ZmanN3Iiwicm9sZSI6ImFub24iLCJpYXQiOjE3MzU1NzMzNzYsImV4cCI6MjA1MTE0OTM3Nn0.FZEr0x7PosVmJJqJ5tZXMJG0Q_6WuNZLjEyUddctYS4';

async function generateApiKey() {
  return 'fnx_' + crypto.randomBytes(32).toString('hex');
}

async function hashPassword(password) {
  return crypto.pbkdf2Sync(password, 'flow-nexus-salt', 100000, 64, 'sha256').toString('hex');
}

async function checkExistingConfig() {
  try {
    const envContent = await fs.readFile(ENV_PATH, 'utf8').catch(() => '');
    const hasUserEmail = envContent.includes('FLOW_NEXUS_USER_EMAIL');
    const hasApiKey = envContent.includes('FLOW_NEXUS_API_KEY');
    
    if (hasUserEmail && hasApiKey) {
      const email = process.env.FLOW_NEXUS_USER_EMAIL;
      const apiKey = process.env.FLOW_NEXUS_API_KEY;
      
      console.log(chalk.yellow('\n📋 Existing configuration found:'));
      console.log(chalk.cyan(`   Email: ${email}`));
      console.log(chalk.cyan(`   API Key: ${apiKey?.substring(0, 10)}...`));
      
      const overwrite = await rl.question(chalk.yellow('\nDo you want to update this configuration? (y/n): '));
      return overwrite.toLowerCase() === 'y';
    }
  } catch (error) {
    // No existing config
  }
  return true;
}

async function registerUser(email, password, username) {
  const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY);
  
  try {
    // Check if user exists
    const { data: existingUser } = await supabase
      .from('user_profiles')
      .select('id, email, api_key')
      .eq('email', email)
      .single();
    
    if (existingUser) {
      console.log(chalk.yellow('\n⚠️  User already exists'));
      
      // Verify password
      const { data: authData, error: authError } = await supabase.auth.signInWithPassword({
        email,
        password
      });
      
      if (authError) {
        throw new Error('Invalid password for existing user');
      }
      
      return {
        userId: existingUser.id,
        email: existingUser.email,
        apiKey: existingUser.api_key || await generateApiKey(),
        isNew: false
      };
    }
    
    // Register new user
    console.log(chalk.green('\n🚀 Registering new user...'));
    
    const { data: authData, error: authError } = await supabase.auth.signUp({
      email,
      password,
      options: {
        data: {
          username,
          skip_email_verification: true
        }
      }
    });
    
    if (authError) throw authError;
    
    const apiKey = await generateApiKey();
    const userId = authData.user?.id;
    
    if (!userId) throw new Error('Failed to create user');
    
    // Create user profile with API key
    const { error: profileError } = await supabase
      .from('user_profiles')
      .insert({
        id: userId,
        email,
        username,
        api_key: apiKey,
        credits: 2560,
        tier: 'free',
        settings: {
          limits: {
            swarms: 3,
            agents: 10,
            tasks: 100,
            sandboxes: 5,
            storage: '1GB'
          }
        }
      });
    
    if (profileError && profileError.code !== '23505') { // Ignore duplicate key error
      console.error('Profile error:', profileError);
    }
    
    return {
      userId,
      email,
      apiKey,
      isNew: true
    };
    
  } catch (error) {
    console.error(chalk.red('\n❌ Registration failed:'), error.message);
    throw error;
  }
}

async function updateEnvFile(config) {
  try {
    // Read existing .env content
    let envContent = await fs.readFile(ENV_PATH, 'utf8').catch(() => '');
    
    // Parse existing variables
    const envVars = new Map();
    envContent.split('\n').forEach(line => {
      const match = line.match(/^([^=]+)=(.*)$/);
      if (match) {
        envVars.set(match[1], match[2]);
      }
    });
    
    // Update with new user configuration
    envVars.set('FLOW_NEXUS_USER_EMAIL', config.email);
    envVars.set('FLOW_NEXUS_USER_ID', config.userId);
    envVars.set('FLOW_NEXUS_API_KEY', config.apiKey);
    envVars.set('FLOW_NEXUS_USER_TIER', config.tier || 'free');
    
    // Ensure required Supabase variables
    if (!envVars.has('SUPABASE_URL')) {
      envVars.set('SUPABASE_URL', SUPABASE_URL);
    }
    if (!envVars.has('SUPABASE_ANON_KEY')) {
      envVars.set('SUPABASE_ANON_KEY', SUPABASE_ANON_KEY);
    }
    if (!envVars.has('SUPABASE_SERVICE_KEY')) {
      // This should be set securely in production
      envVars.set('SUPABASE_SERVICE_KEY', process.env.SUPABASE_SERVICE_KEY || 'YOUR_SERVICE_KEY_HERE');
    }
    if (!envVars.has('JWT_SECRET')) {
      envVars.set('JWT_SECRET', crypto.randomBytes(32).toString('hex'));
    }
    
    // Additional MCP server settings
    envVars.set('MCP_HOST', 'localhost');
    envVars.set('MCP_PORT', '3001');
    envVars.set('WASM_MEMORY_LIMIT', '256MB');
    envVars.set('WASM_SIMD_ENABLED', 'true');
    envVars.set('WASM_THREAD_POOL_SIZE', '4');
    
    // Build new .env content
    const newEnvContent = Array.from(envVars.entries())
      .map(([key, value]) => `${key}=${value}`)
      .join('\n');
    
    // Write to .env file
    await fs.writeFile(ENV_PATH, newEnvContent + '\n', 'utf8');
    
    // Also create .env.local for user-specific overrides
    const localEnvContent = `# User-specific configuration
FLOW_NEXUS_USER_EMAIL=${config.email}
FLOW_NEXUS_USER_ID=${config.userId}
FLOW_NEXUS_API_KEY=${config.apiKey}
FLOW_NEXUS_USER_TIER=${config.tier || 'free'}
`;
    
    await fs.writeFile(ENV_LOCAL_PATH, localEnvContent, 'utf8');
    
    console.log(chalk.green('\n✅ Configuration saved to .env and .env.local'));
    
  } catch (error) {
    console.error(chalk.red('\n❌ Failed to update .env file:'), error);
    throw error;
  }
}

async function main() {
  console.log(chalk.blue.bold('\n🎮 Flow Nexus MCP Server - User Setup\n'));
  console.log(chalk.gray('This will configure your personal MCP server account'));
  console.log(chalk.gray('Your data will be isolated from other users\n'));
  
  try {
    // Check if we should proceed
    const shouldContinue = await checkExistingConfig();
    if (!shouldContinue) {
      console.log(chalk.yellow('\n⚠️  Setup cancelled'));
      process.exit(0);
    }
    
    // Get user input
    console.log(chalk.cyan('\n📝 Please enter your details:\n'));
    
    const email = await rl.question(chalk.white('Email: '));
    if (!email || !email.includes('@')) {
      throw new Error('Invalid email address');
    }
    
    const username = await rl.question(chalk.white('Username: '));
    if (!username || username.length < 3) {
      throw new Error('Username must be at least 3 characters');
    }
    
    const password = await rl.question(chalk.white('Password: '));
    if (!password || password.length < 6) {
      throw new Error('Password must be at least 6 characters');
    }
    
    // Register or authenticate user
    console.log(chalk.yellow('\n🔐 Authenticating...'));
    const userConfig = await registerUser(email, password, username);
    
    // Update .env file
    await updateEnvFile({
      ...userConfig,
      username,
      tier: 'free'
    });
    
    // Display success message
    console.log(chalk.green.bold('\n✨ Setup Complete!\n'));
    
    if (userConfig.isNew) {
      console.log(chalk.cyan('🎉 Welcome to Flow Nexus!'));
      console.log(chalk.cyan('   You have been granted 2560 rUv credits to get started'));
    } else {
      console.log(chalk.cyan('👋 Welcome back!'));
    }
    
    console.log(chalk.white('\n📋 Your Configuration:'));
    console.log(chalk.gray('   Email:    ') + chalk.white(email));
    console.log(chalk.gray('   Username: ') + chalk.white(username));
    console.log(chalk.gray('   API Key:  ') + chalk.white(userConfig.apiKey.substring(0, 20) + '...'));
    console.log(chalk.gray('   User ID:  ') + chalk.white(userConfig.userId));
    
    console.log(chalk.yellow('\n🔒 Security Notes:'));
    console.log(chalk.gray('   • Your API key has been saved to .env and .env.local'));
    console.log(chalk.gray('   • Keep these files secure and never commit them to git'));
    console.log(chalk.gray('   • All your data will be isolated to your account'));
    
    console.log(chalk.cyan('\n🚀 Next Steps:'));
    console.log(chalk.white('   1. Start the MCP server:  ') + chalk.green('npm start'));
    console.log(chalk.white('   2. Install globally:      ') + chalk.green('npm install -g flow-nexus'));
    console.log(chalk.white('   3. Use with Claude:       ') + chalk.green('npx flow-nexus'));
    
  } catch (error) {
    console.error(chalk.red('\n❌ Setup failed:'), error.message);
    process.exit(1);
  } finally {
    rl.close();
  }
}

// Run the script
main().catch(console.error);