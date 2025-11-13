#!/usr/bin/env node

import dotenv from 'dotenv';
import path from 'path';
import { fileURLToPath } from 'url';
import supabaseClient from '../src/services/supabase-client.js';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
dotenv.config({ path: path.join(__dirname, '../.env') });

async function debugSession() {
  console.log('Debugging session...\n');
  
  const { data: { session } } = await supabaseClient.supabase.auth.getSession();
  
  if (session) {
    console.log('Session found:');
    console.log('- User ID:', session.user?.id);
    console.log('- Email:', session.user?.email);
    console.log('- Access Token (first 20 chars):', session.access_token?.substring(0, 20) + '...');
    console.log('- Token type:', typeof session.access_token);
  } else {
    console.log('No session found');
  }
  
  // Check current user
  const user = await supabaseClient.getCurrentUser();
  console.log('\nCurrent user:', user);
  
  // Check profile
  const profile = await supabaseClient.getUserProfile();
  console.log('\nProfile:', profile);
}

debugSession();