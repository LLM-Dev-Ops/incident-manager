/**
 * Centralized Supabase configuration for Flow Nexus
 * These are public credentials that work with Row Level Security (RLS)
 * DO NOT read from environment variables to avoid conflicts with user's local environment
 */

// IMPORTANT: Hardcoded values for consistent behavior across all environments
export const SUPABASE_URL = 'https://pklhxiuouhrcrreectbo.supabase.co';
export const SUPABASE_ANON_KEY = 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InBrbGh4aXVvdWhyY3JyZWVjdGJvIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTU3MDQ1MTQsImV4cCI6MjA3MTI4MDUxNH0.uI34fyRxItPUVKUmn2dc_2RtNxbalHVfmU2EaOV8MK4';

// Export as default object as well for convenience
export default {
  url: SUPABASE_URL,
  anonKey: SUPABASE_ANON_KEY
};