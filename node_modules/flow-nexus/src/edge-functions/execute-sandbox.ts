// Edge Function for secure sandbox execution
import { serve } from "https://deno.land/std@0.168.0/http/server.ts"
import { createClient } from 'https://esm.sh/@supabase/supabase-js@2'

const corsHeaders = {
  'Access-Control-Allow-Origin': '*',
  'Access-Control-Allow-Headers': 'authorization, x-client-info, apikey, content-type',
}

serve(async (req) => {
  // Handle CORS
  if (req.method === 'OPTIONS') {
    return new Response('ok', { headers: corsHeaders })
  }

  try {
    const { code, language, env_vars, sandbox_id } = await req.json()

    // Get Supabase client
    const supabaseUrl = Deno.env.get('SUPABASE_URL')!
    const supabaseKey = Deno.env.get('SUPABASE_SERVICE_ROLE_KEY')!
    const supabase = createClient(supabaseUrl, supabaseKey)

    // Get API keys from Supabase Vault
    const { data: secrets, error: vaultError } = await supabase
      .from('vault')
      .select('*')
      .eq('name', 'api_keys')
      .single()

    if (vaultError) {
      throw new Error('Failed to retrieve API keys from vault')
    }

    // Merge environment variables with secrets
    const finalEnvVars = {
      ...env_vars,
      ...secrets.value
    }

    // Execute code based on language
    let result
    if (language === 'javascript' || language === 'typescript') {
      // Create a new function with the code
      const AsyncFunction = Object.getPrototypeOf(async function(){}).constructor
      const fn = new AsyncFunction('env', code)
      result = await fn(finalEnvVars)
    } else {
      throw new Error(`Unsupported language: ${language}`)
    }

    // Store execution result
    const { error: insertError } = await supabase
      .from('sandbox_executions')
      .insert({
        sandbox_id,
        code,
        language,
        result: JSON.stringify(result),
        status: 'completed',
        executed_at: new Date().toISOString()
      })

    if (insertError) {
      console.error('Failed to store execution result:', insertError)
    }

    return new Response(
      JSON.stringify({
        success: true,
        result,
        execution_id: crypto.randomUUID()
      }),
      {
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        status: 200,
      }
    )
  } catch (error) {
    return new Response(
      JSON.stringify({
        success: false,
        error: error.message
      }),
      {
        headers: { ...corsHeaders, 'Content-Type': 'application/json' },
        status: 400,
      }
    )
  }
})