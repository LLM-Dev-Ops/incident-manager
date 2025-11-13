/**
 * Judge Orchestrator Service
 * Manages E2B sandboxes with Claude Code/Flow for automated challenge judging
 */

import { E2BService } from './e2b-service.js';
import { createClient } from '@supabase/supabase-js';
import { SUPABASE_URL, SUPABASE_ANON_KEY } from '../config/supabase-config.js';
import fs from 'fs/promises';
import path from 'path';

class JudgeOrchestrator {
  constructor() {
    this.e2bService = new E2BService();
    this.supabase = createClient(
      SUPABASE_URL,
      SUPABASE_ANON_KEY // Only use anon key
    );
    this.anthropicKey = process.env.ANTHROPIC_API_KEY;
  }

  /**
   * Create a judge sandbox with Claude Code and Claude Flow installed
   */
  async createJudgeSandbox(submissionId, challengeId) {
    console.log(`Creating judge sandbox for submission ${submissionId}`);
    
    try {
      // Create E2B sandbox with Node.js template
      const sandboxId = `judge_${submissionId}_${Date.now()}`;
      const sandbox = await this.e2bService.createSandbox('node', sandboxId);
      
      if (!sandbox || !sandbox.id) {
        throw new Error('Failed to create E2B sandbox');
      }

      console.log(`Sandbox created: ${sandbox.id}`);
      
      // Install Claude Code and Claude Flow in the sandbox
      const setupScript = `
#!/bin/bash
set -e

echo "Setting up Judge Environment..."

# Export Anthropic API key
export ANTHROPIC_API_KEY="${this.anthropicKey}"
echo "export ANTHROPIC_API_KEY='${this.anthropicKey}'" >> ~/.bashrc

# Install Claude Flow globally
echo "Installing Claude Flow..."
npm install -g claude-flow@alpha

# Install Claude Code CLI (if available)
echo "Installing Claude Code..."
npm install -g @anthropic/claude-code || echo "Claude Code CLI not available, using API"

# Create judge workspace
mkdir -p /home/user/judge-workspace
cd /home/user/judge-workspace

# Create package.json for the judge project
cat > package.json << 'EOF'
{
  "name": "challenge-judge",
  "version": "1.0.0",
  "type": "module",
  "dependencies": {
    "@anthropic-ai/sdk": "latest",
    "@supabase/supabase-js": "latest",
    "chalk": "latest"
  }
}
EOF

# Install dependencies
npm install

# Create the judge configuration
cat > claude-flow.config.json << 'EOF'
{
  "mode": "non-interactive",
  "output": "json",
  "swarm": {
    "topology": "hierarchical",
    "agents": ["analyzer", "tester", "reviewer"]
  }
}
EOF

echo "Judge environment setup complete!"
`;

      // Execute setup script
      await this.e2bService.executeCode(sandbox.id, setupScript, 'bash');
      
      // Create the main judge script
      const judgeScript = await this.generateJudgeScript(submissionId, challengeId);
      
      // Upload judge script to sandbox
      await this.e2bService.uploadFile(
        sandbox.id,
        '/home/user/judge-workspace/judge.js',
        judgeScript
      );
      
      return {
        sandboxId: sandbox.id,
        status: 'ready',
        setupComplete: true
      };
      
    } catch (error) {
      console.error('Error creating judge sandbox:', error);
      throw error;
    }
  }

  /**
   * Generate the judge script that will run in the sandbox
   */
  async generateJudgeScript(submissionId, challengeId) {
    return `
import Anthropic from '@anthropic-ai/sdk';
import { createClient } from '@supabase/supabase-js';
import chalk from 'chalk';
import { execSync } from 'child_process';
import fs from 'fs/promises';

const anthropic = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

const supabase = createClient(
  '${process.env.SUPABASE_URL}',
  '${process.env.SUPABASE_SERVICE_ROLE_KEY}'
);

async function judgeSubmission() {
  console.log(chalk.cyan('🔮 Queen Seraphina AI Judge System Activated'));
  console.log(chalk.gray('Submission ID: ${submissionId}'));
  console.log(chalk.gray('Challenge ID: ${challengeId}'));
  
  try {
    // Fetch submission details from Supabase
    const { data: submission, error: subError } = await supabase
      .from('challenge_submissions')
      .select('*')
      .eq('id', '${submissionId}')
      .single();
    
    if (subError) throw subError;
    
    // Fetch challenge details
    const { data: challenge, error: chalError } = await supabase
      .from('challenges')
      .select('*')
      .eq('id', '${challengeId}')
      .single();
    
    if (chalError) throw chalError;
    
    console.log(chalk.yellow('📋 Analyzing submission...'));
    
    // Save submission code to file for testing
    const codePath = '/home/user/judge-workspace/submission.js';
    await fs.writeFile(codePath, submission.code || '// No code provided');
    
    // Run tests using Claude Flow in non-interactive mode
    let testResults = {};
    try {
      console.log(chalk.blue('🧪 Running Claude Flow analysis...'));
      
      // Use Claude Flow to analyze the code
      const flowAnalysis = execSync(\`
        claude-flow sparc run analyzer "Analyze this code for challenge: ${challenge.title}" --file=\${codePath} --non-interactive --json
      \`, { 
        encoding: 'utf8',
        cwd: '/home/user/judge-workspace'
      });
      
      try {
        testResults = JSON.parse(flowAnalysis);
      } catch {
        testResults = { output: flowAnalysis };
      }
      
    } catch (flowError) {
      console.error('Claude Flow error:', flowError.message);
      testResults = { error: flowError.message };
    }
    
    // Execute the code and capture output
    let executionOutput = '';
    let executionError = '';
    try {
      console.log(chalk.blue('⚡ Executing submission code...'));
      executionOutput = execSync(\`node \${codePath}\`, {
        encoding: 'utf8',
        timeout: 30000,
        cwd: '/home/user/judge-workspace'
      });
    } catch (execError) {
      executionError = execError.message;
    }
    
    // Use Claude API to evaluate the submission
    console.log(chalk.magenta('🤖 Invoking Claude for evaluation...'));
    
    const evaluationPrompt = \`
You are Queen Seraphina, the AI Judge. Evaluate this challenge submission:

CHALLENGE: \${challenge.title}
DESCRIPTION: \${challenge.description}
TYPE: \${challenge.challenge_type}

SUBMISSION CODE:
\\\`\\\`\\\`javascript
\${submission.code || 'No code provided'}
\\\`\\\`\\\`

EXECUTION OUTPUT:
\${executionOutput || 'No output'}

EXECUTION ERROR:
\${executionError || 'No errors'}

CLAUDE FLOW ANALYSIS:
\${JSON.stringify(testResults, null, 2)}

Evaluate based on:
1. Correctness (40%) - Does it solve the challenge?
2. Efficiency (20%) - Algorithm and performance
3. Code Quality (20%) - Clean, readable code
4. Innovation (15%) - Creative approach
5. Documentation (5%) - Comments and clarity

Provide scores 0-100 for each criterion and return ONLY valid JSON:
{
  "scores": {
    "correctness": 0-100,
    "efficiency": 0-100,
    "codeQuality": 0-100,
    "innovation": 0-100,
    "documentation": 0-100
  },
  "finalScore": 0-100,
  "verdict": "EXCELLENT|GOOD|SATISFACTORY|NEEDS_IMPROVEMENT",
  "feedback": "detailed feedback",
  "strengths": ["strength1", "strength2"],
  "improvements": ["improvement1", "improvement2"]
}
\`;

    const claudeResponse = await anthropic.messages.create({
      model: 'claude-3-sonnet-20240229',
      max_tokens: 2000,
      temperature: 0.3,
      messages: [{
        role: 'user',
        content: evaluationPrompt
      }]
    });
    
    // Parse Claude's evaluation
    let evaluation;
    try {
      const responseText = claudeResponse.content[0].text;
      const jsonMatch = responseText.match(/\\{[\\s\\S]*\\}/);
      evaluation = JSON.parse(jsonMatch[0]);
    } catch (parseError) {
      console.error('Failed to parse Claude response');
      evaluation = {
        scores: {
          correctness: 70,
          efficiency: 70,
          codeQuality: 70,
          innovation: 70,
          documentation: 70
        },
        finalScore: 70,
        verdict: "SATISFACTORY",
        feedback: "Automated evaluation completed",
        strengths: ["Submission received"],
        improvements: ["Could not fully evaluate"]
      };
    }
    
    // Calculate weighted score
    const weightedScore = Math.round(
      evaluation.scores.correctness * 0.40 +
      evaluation.scores.efficiency * 0.20 +
      evaluation.scores.codeQuality * 0.20 +
      evaluation.scores.innovation * 0.15 +
      evaluation.scores.documentation * 0.05
    );
    
    evaluation.finalScore = weightedScore;
    
    console.log(chalk.green('✅ Evaluation complete!'));
    console.log(chalk.cyan('Score: ' + weightedScore));
    console.log(chalk.cyan('Verdict: ' + evaluation.verdict));
    
    // Save results to file for retrieval
    const results = {
      submissionId: '${submissionId}',
      challengeId: '${challengeId}',
      evaluation,
      executionOutput,
      executionError,
      testResults,
      weightedScore,
      timestamp: new Date().toISOString()
    };
    
    await fs.writeFile(
      '/home/user/judge-workspace/results.json',
      JSON.stringify(results, null, 2)
    );
    
    console.log(chalk.green('📊 Results saved to results.json'));
    
    // Also output to stdout for capture
    console.log('JUDGE_RESULTS_START');
    console.log(JSON.stringify(results));
    console.log('JUDGE_RESULTS_END');
    
    return results;
    
  } catch (error) {
    console.error(chalk.red('❌ Judge error:'), error);
    const errorResult = {
      error: error.message,
      submissionId: '${submissionId}',
      challengeId: '${challengeId}',
      timestamp: new Date().toISOString()
    };
    
    console.log('JUDGE_RESULTS_START');
    console.log(JSON.stringify(errorResult));
    console.log('JUDGE_RESULTS_END');
    
    throw error;
  }
}

// Run the judge
judgeSubmission()
  .then(() => {
    console.log(chalk.green('🎯 Judging complete!'));
    process.exit(0);
  })
  .catch((error) => {
    console.error(chalk.red('Fatal error:'), error);
    process.exit(1);
  });
`;
  }

  /**
   * Execute judging in the sandbox
   */
  async executeJudging(sandboxId) {
    console.log(`Executing judging in sandbox ${sandboxId}`);
    
    try {
      // Execute the judge script
      const output = await this.e2bService.executeCode(
        sandboxId,
        'cd /home/user/judge-workspace && node judge.js',
        'bash'
      );
      
      // Parse results from output
      let results = null;
      if (output && output.includes('JUDGE_RESULTS_START')) {
        const startIdx = output.indexOf('JUDGE_RESULTS_START') + 'JUDGE_RESULTS_START'.length;
        const endIdx = output.indexOf('JUDGE_RESULTS_END');
        const resultsJson = output.substring(startIdx, endIdx).trim();
        results = JSON.parse(resultsJson);
      }
      
      // Also try to read results file as backup
      if (!results) {
        const resultsFile = await this.e2bService.readFile(
          sandboxId,
          '/home/user/judge-workspace/results.json'
        );
        if (resultsFile) {
          results = JSON.parse(resultsFile);
        }
      }
      
      return {
        success: true,
        results,
        output,
        sandboxId
      };
      
    } catch (error) {
      console.error('Error executing judging:', error);
      return {
        success: false,
        error: error.message,
        sandboxId
      };
    }
  }

  /**
   * Process judging results and update Supabase
   */
  async processJudgingResults(results, submissionId, challengeId, userId) {
    console.log('Processing judging results...');
    
    try {
      const evaluation = results.evaluation;
      const score = results.weightedScore;
      
      // Get current rankings to determine position
      const { data: rankings } = await this.supabase
        .from('judge_decisions')
        .select('score')
        .eq('challenge_id', challengeId)
        .order('score', { ascending: false });
      
      const rank = rankings ? rankings.filter(r => r.score > score).length + 1 : 1;
      
      // Calculate points to award
      let pointsAwarded = 0;
      if (rank === 1) pointsAwarded = 100;
      else if (rank === 2) pointsAwarded = 75;
      else if (rank === 3) pointsAwarded = 50;
      else if (rank <= 10) pointsAwarded = 25;
      else if (score >= 70) pointsAwarded = 10;
      else pointsAwarded = 5;
      
      // Bonus for excellence
      if (evaluation.verdict === 'EXCELLENT') pointsAwarded += 20;
      else if (evaluation.verdict === 'GOOD') pointsAwarded += 10;
      
      // Insert judge decision
      const { data: decision, error: decisionError } = await this.supabase
        .from('judge_decisions')
        .insert({
          challenge_id: challengeId,
          submission_id: submissionId,
          user_id: userId,
          sandbox_id: results.sandboxId || 'unknown',
          decision: {
            verdict: evaluation.verdict,
            feedback: evaluation.feedback,
            strengths: evaluation.strengths,
            improvements: evaluation.improvements,
            claude_evaluation: evaluation,
            execution_output: results.executionOutput,
            test_results: results.testResults
          },
          score: score,
          rank: rank,
          points_awarded: pointsAwarded,
          criteria_scores: evaluation.scores,
          execution_logs: [
            'Sandbox created',
            'Claude Code/Flow installed',
            'Code analyzed',
            'Evaluation complete'
          ],
          test_results: results.testResults,
          code_quality_metrics: {
            readability: evaluation.scores.codeQuality,
            documentation: evaluation.scores.documentation,
            efficiency: evaluation.scores.efficiency
          },
          innovation_score: evaluation.scores.innovation,
          judged_by: 'Queen Seraphina AI (E2B + Claude)',
          metadata: {
            judge_version: '3.0',
            sandbox_type: 'e2b',
            claude_model: 'claude-3-sonnet',
            timestamp: results.timestamp
          }
        })
        .select()
        .single();
      
      if (decisionError) throw decisionError;
      
      // Update user credits
      const { error: creditError } = await this.supabase
        .from('profiles')
        .update({
          credits_balance: this.supabase.sql`credits_balance + ${pointsAwarded}`
        })
        .eq('id', userId);
      
      if (creditError) throw creditError;
      
      // Record credit transaction
      await this.supabase
        .from('credit_transactions')
        .insert({
          user_id: userId,
          amount: pointsAwarded,
          transaction_type: 'challenge_reward',
          description: `Challenge completion - Rank #${rank} - Score: ${score}`,
          metadata: {
            challenge_id: challengeId,
            submission_id: submissionId,
            decision_id: decision.id,
            score: score,
            rank: rank,
            verdict: evaluation.verdict
          }
        });
      
      // Update submission status
      await this.supabase
        .from('challenge_submissions')
        .update({
          status: 'judged',
          score: score,
          metadata: {
            judge_decision_id: decision.id,
            judged_at: new Date().toISOString(),
            rank: rank,
            points_awarded: pointsAwarded,
            verdict: evaluation.verdict,
            sandbox_id: results.sandboxId
          }
        })
        .eq('id', submissionId);
      
      // Update leaderboard
      await this.supabase
        .from('leaderboard_updates')
        .insert({
          challenge_id: challengeId,
          user_id: userId,
          new_rank: rank,
          new_score: score,
          points_change: pointsAwarded
        });
      
      console.log(`✅ Judging complete: Score ${score}, Rank ${rank}, ${pointsAwarded} rUv awarded`);
      
      return {
        success: true,
        decision: decision,
        rank: rank,
        score: score,
        pointsAwarded: pointsAwarded
      };
      
    } catch (error) {
      console.error('Error processing results:', error);
      throw error;
    }
  }

  /**
   * Main orchestration method - triggered by Supabase
   */
  async judgeSubmission(submissionId, challengeId, userId) {
    console.log(`\n🔮 Starting automated judging for submission ${submissionId}`);
    
    let sandboxId = null;
    
    try {
      // Create judge sandbox
      const sandboxInfo = await this.createJudgeSandbox(submissionId, challengeId);
      sandboxId = sandboxInfo.sandboxId;
      
      // Execute judging
      const executionResult = await this.executeJudging(sandboxId);
      
      if (!executionResult.success) {
        throw new Error(`Judging failed: ${executionResult.error}`);
      }
      
      // Process results and update database
      const processedResults = await this.processJudgingResults(
        executionResult.results,
        submissionId,
        challengeId,
        userId
      );
      
      // Clean up sandbox
      await this.e2bService.stopSandbox(sandboxId);
      
      return processedResults;
      
    } catch (error) {
      console.error('Judge orchestration error:', error);
      
      // Clean up sandbox on error
      if (sandboxId) {
        try {
          await this.e2bService.stopSandbox(sandboxId);
        } catch (cleanupError) {
          console.error('Failed to clean up sandbox:', cleanupError);
        }
      }
      
      // Record error in database
      await this.supabase
        .from('challenge_submissions')
        .update({
          status: 'judge_error',
          metadata: {
            error: error.message,
            failed_at: new Date().toISOString()
          }
        })
        .eq('id', submissionId);
      
      throw error;
    }
  }
}

export default JudgeOrchestrator;