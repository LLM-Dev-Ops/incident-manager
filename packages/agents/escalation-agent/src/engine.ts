/**
 * EscalationDecisionEngine
 *
 * Core decision logic for the Escalation Agent.
 * This implements deterministic escalation evaluation.
 */

import {
  SEVERITY_NUMERIC,
  type Severity,
  type EscalationAgentInput,
  type EscalationAgentOutput,
  type EscalationDecision,
  type OrchestratorAction,
  type AppliedPolicy,
  type EvaluationDetails,
  type ThresholdEvaluation,
  type TimeFactor,
  type ConfidenceFactor
} from '@agentics/contracts';
import type { AgentConfig } from './config.js';

// ============================================================================
// ENGINE
// ============================================================================

export class EscalationDecisionEngine {
  constructor(private readonly config: AgentConfig) {}

  /**
   * Evaluate an incident for escalation
   *
   * This is the core decision logic. It MUST be:
   * - Deterministic
   * - Stateless
   * - Return exactly one decision
   */
  evaluate(input: EscalationAgentInput): EscalationResult {
    const startTime = Date.now();

    // 1. Evaluate all thresholds
    const thresholds = this.evaluateThresholds(input);

    // 2. Calculate time factors
    const timeFactors = this.calculateTimeFactors(input);

    // 3. Calculate raw escalation score
    const rawScore = this.calculateRawScore(thresholds, timeFactors, input);

    // 4. Normalize score to 0-1
    const normalizedScore = Math.min(1, Math.max(0, rawScore));

    // 5. Make decision based on score and constraints
    const decision = this.makeDecision(normalizedScore, input);

    // 6. Calculate confidence
    const { confidence, factors } = this.calculateConfidence(
      decision,
      thresholds,
      timeFactors,
      input
    );

    // 7. Determine new severity if escalating/deescalating
    const { newSeverity, severityDelta, newLevel } = this.calculateSeverityChange(
      decision,
      input
    );

    // 8. Generate orchestrator actions
    const actions = this.generateActions(decision, input, newSeverity, newLevel);

    // 9. Build evaluation details
    const evaluationDetails: EvaluationDetails = {
      thresholds_evaluated: thresholds,
      time_factors: timeFactors,
      raw_escalation_score: rawScore,
      normalized_score: normalizedScore
    };

    // 10. Build applied policy
    const appliedPolicy = this.buildAppliedPolicy(input);

    // 11. Build output
    const output: EscalationAgentOutput = {
      decision,
      reason: this.generateReason(decision, thresholds, timeFactors, input),
      new_severity: newSeverity,
      new_escalation_level: newLevel,
      severity_delta: severityDelta,
      orchestrator_actions: actions,
      next_evaluation_at: this.calculateNextEvaluation(decision, input),
      applied_policy: appliedPolicy,
      evaluation_details: evaluationDetails
    };

    if (decision === 'defer') {
      output.defer_until = this.calculateDeferUntil(input);
    }

    const processingTimeMs = Date.now() - startTime;

    return {
      output,
      confidence,
      confidenceFactors: factors,
      processingTimeMs
    };
  }

  // ============================================================================
  // THRESHOLD EVALUATION
  // ============================================================================

  private evaluateThresholds(input: EscalationAgentInput): ThresholdEvaluation[] {
    const thresholds: ThresholdEvaluation[] = [];

    // SLA acknowledgment threshold
    if (input.sla.acknowledgment_deadline) {
      const deadline = new Date(input.sla.acknowledgment_deadline).getTime();
      const now = Date.now();
      const remainingMs = deadline - now;
      const remainingMinutes = remainingMs / 60000;

      thresholds.push({
        name: 'sla_acknowledgment',
        threshold_value: 0, // Should be above 0 minutes
        actual_value: remainingMinutes,
        breached: input.sla.acknowledgment_breached || remainingMinutes <= 0,
        weight: 0.3
      });
    }

    // SLA resolution threshold
    if (input.sla.resolution_deadline) {
      const deadline = new Date(input.sla.resolution_deadline).getTime();
      const now = Date.now();
      const remainingMs = deadline - now;
      const remainingMinutes = remainingMs / 60000;

      thresholds.push({
        name: 'sla_resolution',
        threshold_value: 30, // Should have > 30 minutes remaining
        actual_value: remainingMinutes,
        breached: input.sla.resolution_breached || remainingMinutes <= 30,
        weight: 0.25
      });
    }

    // Severity-based threshold (should escalate if signal suggests higher severity)
    if (input.signal_payload.suggested_severity) {
      const currentSeverityNum = SEVERITY_NUMERIC[input.current_severity];
      const suggestedSeverityNum = SEVERITY_NUMERIC[input.signal_payload.suggested_severity];
      const severityDiff = currentSeverityNum - suggestedSeverityNum;

      thresholds.push({
        name: 'signal_severity_suggestion',
        threshold_value: 0, // No escalation needed if diff <= 0
        actual_value: severityDiff,
        breached: severityDiff > 0, // Current is less severe than suggested
        weight: 0.3
      });
    }

    // Signal confidence threshold
    if (input.signal_payload.signal_confidence !== undefined) {
      thresholds.push({
        name: 'signal_confidence',
        threshold_value: 0.7, // Should be > 70% confident
        actual_value: input.signal_payload.signal_confidence,
        breached: input.signal_payload.signal_confidence >= 0.7,
        weight: 0.15
      });
    }

    // Current escalation level threshold
    thresholds.push({
      name: 'escalation_level_cap',
      threshold_value: this.config.escalation.maxEscalationLevel,
      actual_value: input.current_escalation_level,
      breached: input.current_escalation_level >= this.config.escalation.maxEscalationLevel,
      weight: 0.0 // This is a hard constraint, not a score factor
    });

    return thresholds;
  }

  // ============================================================================
  // TIME FACTORS
  // ============================================================================

  private calculateTimeFactors(input: EscalationAgentInput): TimeFactor[] {
    const factors: TimeFactor[] = [];
    const now = Date.now();

    // Time since creation
    const createdAt = new Date(input.incident_created_at).getTime();
    const timeSinceCreation = (now - createdAt) / 1000;
    factors.push({
      name: 'time_since_creation',
      value_seconds: timeSinceCreation,
      score_contribution: this.timeToScore(timeSinceCreation, 'creation')
    });

    // Time since last escalation
    if (input.time_since_last_escalation !== undefined) {
      factors.push({
        name: 'time_since_last_escalation',
        value_seconds: input.time_since_last_escalation,
        score_contribution: this.timeToScore(input.time_since_last_escalation, 'escalation')
      });
    }

    // Time in current state
    const updatedAt = new Date(input.incident_updated_at).getTime();
    const timeInCurrentState = (now - updatedAt) / 1000;
    factors.push({
      name: 'time_in_current_state',
      value_seconds: timeInCurrentState,
      score_contribution: this.timeToScore(timeInCurrentState, 'state')
    });

    // SLA remaining time
    if (input.sla.resolution_deadline) {
      const deadline = new Date(input.sla.resolution_deadline).getTime();
      const slaRemaining = (deadline - now) / 1000;
      factors.push({
        name: 'sla_remaining',
        value_seconds: slaRemaining,
        score_contribution: this.slaToScore(slaRemaining)
      });
    }

    return factors;
  }

  private timeToScore(seconds: number, type: 'creation' | 'escalation' | 'state'): number {
    // Different time thresholds based on severity would apply here
    // For now, use generic thresholds
    const thresholds = {
      creation: [300, 900, 1800, 3600], // 5m, 15m, 30m, 1h
      escalation: [300, 600, 1200, 2400], // 5m, 10m, 20m, 40m
      state: [180, 600, 1200, 2400] // 3m, 10m, 20m, 40m
    };

    const t = thresholds[type];
    if (seconds < t[0]) return 0;
    if (seconds < t[1]) return 0.1;
    if (seconds < t[2]) return 0.2;
    if (seconds < t[3]) return 0.3;
    return 0.4;
  }

  private slaToScore(remainingSeconds: number): number {
    if (remainingSeconds <= 0) return 0.5; // SLA breached - high urgency
    if (remainingSeconds < 300) return 0.4; // < 5 minutes
    if (remainingSeconds < 900) return 0.3; // < 15 minutes
    if (remainingSeconds < 1800) return 0.2; // < 30 minutes
    if (remainingSeconds < 3600) return 0.1; // < 1 hour
    return 0;
  }

  // ============================================================================
  // SCORE CALCULATION
  // ============================================================================

  private calculateRawScore(
    thresholds: ThresholdEvaluation[],
    timeFactors: TimeFactor[],
    input: EscalationAgentInput
  ): number {
    let score = 0;

    // Add threshold contributions
    for (const t of thresholds) {
      if (t.breached && t.weight > 0) {
        score += t.weight;
      }
    }

    // Add time factor contributions
    for (const f of timeFactors) {
      score += f.score_contribution;
    }

    // Add severity weight (more severe = more likely to escalate)
    const severityWeight = (4 - SEVERITY_NUMERIC[input.current_severity]) * 0.05;
    score += severityWeight;

    // Add signal type weight
    if (input.signal_payload.type === 'threshold_breach') {
      score += 0.15;
    } else if (input.signal_payload.type === 'anomaly') {
      score += 0.1;
    } else if (input.signal_payload.type === 'timeout') {
      score += 0.2;
    }

    return score;
  }

  // ============================================================================
  // DECISION MAKING
  // ============================================================================

  private makeDecision(
    normalizedScore: number,
    input: EscalationAgentInput
  ): EscalationDecision {
    // Hard constraints - cannot escalate
    if (input.current_escalation_level >= this.config.escalation.maxEscalationLevel) {
      return 'maintain';
    }

    // Check minimum escalation interval
    if (input.time_since_last_escalation !== undefined &&
        input.time_since_last_escalation < this.config.escalation.minEscalationInterval) {
      return 'defer';
    }

    // Already resolved/closed - no escalation
    if (input.current_status === 'RESOLVED' || input.current_status === 'CLOSED') {
      return 'maintain';
    }

    // Decision based on score thresholds
    if (normalizedScore >= 0.8) {
      return 'escalate';
    } else if (normalizedScore >= 0.5) {
      // Medium score - check if signal suggests escalation
      if (input.signal_payload.suggested_severity) {
        const suggestedNum = SEVERITY_NUMERIC[input.signal_payload.suggested_severity];
        const currentNum = SEVERITY_NUMERIC[input.current_severity];
        if (suggestedNum < currentNum) {
          return 'escalate';
        }
      }
      return 'maintain';
    } else if (normalizedScore <= 0.2) {
      // Low score - consider deescalation if not at lowest level
      if (input.current_escalation_level > 0 &&
          input.current_severity !== 'P4' &&
          input.current_status === 'IN_PROGRESS') {
        return 'deescalate';
      }
    }

    return 'maintain';
  }

  // ============================================================================
  // CONFIDENCE CALCULATION
  // ============================================================================

  private calculateConfidence(
    decision: EscalationDecision,
    thresholds: ThresholdEvaluation[],
    timeFactors: TimeFactor[],
    input: EscalationAgentInput
  ): { confidence: number; factors: ConfidenceFactor[] } {
    const factors: ConfidenceFactor[] = [];
    let totalWeight = 0;
    let weightedSum = 0;

    // Factor 1: Threshold agreement
    const breachedCount = thresholds.filter(t => t.breached && t.weight > 0).length;
    const totalThresholds = thresholds.filter(t => t.weight > 0).length;
    const thresholdAgreement = totalThresholds > 0 ? breachedCount / totalThresholds : 0.5;

    factors.push({
      factor: 'threshold_agreement',
      weight: 0.3,
      contribution: thresholdAgreement * 0.3,
      explanation: `${breachedCount}/${totalThresholds} thresholds breached`
    });
    totalWeight += 0.3;
    weightedSum += thresholdAgreement * 0.3;

    // Factor 2: Signal confidence
    const signalConfidence = input.signal_payload.signal_confidence ?? 0.5;
    factors.push({
      factor: 'signal_confidence',
      weight: 0.25,
      contribution: signalConfidence * 0.25,
      explanation: `Signal confidence: ${(signalConfidence * 100).toFixed(0)}%`
    });
    totalWeight += 0.25;
    weightedSum += signalConfidence * 0.25;

    // Factor 3: Historical pattern match
    const historyMatch = this.calculateHistoryMatch(input);
    factors.push({
      factor: 'history_match',
      weight: 0.2,
      contribution: historyMatch * 0.2,
      explanation: `History pattern match: ${(historyMatch * 100).toFixed(0)}%`
    });
    totalWeight += 0.2;
    weightedSum += historyMatch * 0.2;

    // Factor 4: Decision clarity (how clear-cut is the decision)
    const decisionClarity = decision === 'maintain' ? 0.7 : 0.9;
    factors.push({
      factor: 'decision_clarity',
      weight: 0.15,
      contribution: decisionClarity * 0.15,
      explanation: `Decision: ${decision} has clear justification`
    });
    totalWeight += 0.15;
    weightedSum += decisionClarity * 0.15;

    // Factor 5: Data completeness
    const dataCompleteness = this.calculateDataCompleteness(input);
    factors.push({
      factor: 'data_completeness',
      weight: 0.1,
      contribution: dataCompleteness * 0.1,
      explanation: `Input data completeness: ${(dataCompleteness * 100).toFixed(0)}%`
    });
    totalWeight += 0.1;
    weightedSum += dataCompleteness * 0.1;

    const confidence = totalWeight > 0 ? weightedSum / totalWeight : 0.5;

    return { confidence: Math.min(1, Math.max(0, confidence)), factors };
  }

  private calculateHistoryMatch(input: EscalationAgentInput): number {
    if (input.escalation_history.length === 0) {
      return 0.5; // No history - neutral
    }

    // Check if recent escalations had similar patterns
    const recentEscalations = input.escalation_history.slice(-3);
    let matchScore = 0;

    for (const entry of recentEscalations) {
      // Same severity transition direction
      if (entry.to_severity === input.signal_payload.suggested_severity) {
        matchScore += 0.3;
      }
    }

    return Math.min(1, 0.5 + matchScore);
  }

  private calculateDataCompleteness(input: EscalationAgentInput): number {
    let completeness = 0;
    const checks = [
      input.incident_id,
      input.fingerprint,
      input.current_severity,
      input.current_status,
      input.signal_source,
      input.signal_timestamp,
      input.affected_resource,
      input.title,
      input.incident_created_at,
      input.sla
    ];

    for (const check of checks) {
      if (check !== undefined && check !== null) {
        completeness += 1 / checks.length;
      }
    }

    return completeness;
  }

  // ============================================================================
  // SEVERITY CHANGE
  // ============================================================================

  private calculateSeverityChange(
    decision: EscalationDecision,
    input: EscalationAgentInput
  ): { newSeverity?: Severity; severityDelta?: number; newLevel?: number } {
    if (decision === 'maintain' || decision === 'defer') {
      return {};
    }

    const currentNum = SEVERITY_NUMERIC[input.current_severity];

    if (decision === 'escalate') {
      // Escalate: decrease severity number (more severe)
      const newNum = Math.max(0, currentNum - 1);
      const newSeverity = this.numToSeverity(newNum);
      const newLevel = Math.min(
        input.current_escalation_level + 1,
        this.config.escalation.maxEscalationLevel
      );

      return {
        newSeverity,
        severityDelta: newNum - currentNum, // Negative = escalation
        newLevel
      };
    }

    if (decision === 'deescalate') {
      // Deescalate: increase severity number (less severe)
      const newNum = Math.min(4, currentNum + 1);
      const newSeverity = this.numToSeverity(newNum);
      const newLevel = Math.max(0, input.current_escalation_level - 1);

      return {
        newSeverity,
        severityDelta: newNum - currentNum, // Positive = deescalation
        newLevel
      };
    }

    return {};
  }

  private numToSeverity(num: number): Severity {
    const map: Record<number, Severity> = { 0: 'P0', 1: 'P1', 2: 'P2', 3: 'P3', 4: 'P4' };
    return map[num] ?? 'P4';
  }

  // ============================================================================
  // ACTION GENERATION
  // ============================================================================

  private generateActions(
    decision: EscalationDecision,
    input: EscalationAgentInput,
    newSeverity?: Severity,
    newLevel?: number
  ): OrchestratorAction[] {
    const actions: OrchestratorAction[] = [];

    // Always log timeline event
    actions.push({
      action_type: 'log_timeline_event',
      priority: 'normal',
      parameters: {
        incident_id: input.incident_id,
        event_type: 'escalation_evaluation',
        decision,
        from_severity: input.current_severity,
        to_severity: newSeverity,
        from_level: input.current_escalation_level,
        to_level: newLevel
      },
      async: true
    });

    if (decision === 'escalate') {
      // Update incident status
      actions.push({
        action_type: 'update_incident_status',
        priority: 'high',
        parameters: {
          incident_id: input.incident_id,
          new_status: 'ESCALATED',
          new_severity: newSeverity,
          escalation_level: newLevel
        },
        async: false
      });

      // Notify escalation targets
      actions.push({
        action_type: 'notify_escalation_targets',
        priority: newSeverity === 'P0' ? 'critical' : 'high',
        parameters: {
          incident_id: input.incident_id,
          escalation_level: newLevel,
          severity: newSeverity
        },
        async: true
      });

      // Request approval for P0
      if (newSeverity === 'P0' && this.config.escalation.requireApprovalForP0) {
        actions.push({
          action_type: 'request_approval',
          priority: 'critical',
          parameters: {
            incident_id: input.incident_id,
            approval_type: 'p0_escalation',
            reason: `Escalation to P0 requires approval`
          },
          async: false
        });
      }
    } else if (decision === 'deescalate') {
      // Update incident status
      actions.push({
        action_type: 'update_incident_status',
        priority: 'normal',
        parameters: {
          incident_id: input.incident_id,
          new_severity: newSeverity,
          escalation_level: newLevel
        },
        async: false
      });
    }

    return actions;
  }

  // ============================================================================
  // UTILITY METHODS
  // ============================================================================

  private calculateNextEvaluation(
    decision: EscalationDecision,
    input: EscalationAgentInput
  ): string | undefined {
    if (decision === 'defer') {
      return undefined; // defer_until is set separately
    }

    // Schedule next evaluation based on severity
    const intervals: Record<Severity, number> = {
      'P0': 60, // 1 minute
      'P1': 180, // 3 minutes
      'P2': 300, // 5 minutes
      'P3': 600, // 10 minutes
      'P4': 1800 // 30 minutes
    };

    const severity = decision === 'escalate' ?
      (this.numToSeverity(Math.max(0, SEVERITY_NUMERIC[input.current_severity] - 1))) :
      input.current_severity;

    const intervalSeconds = intervals[severity];
    const nextEval = new Date(Date.now() + intervalSeconds * 1000);

    return nextEval.toISOString();
  }

  private calculateDeferUntil(input: EscalationAgentInput): string {
    // Defer until minimum escalation interval has passed
    const deferSeconds = this.config.escalation.minEscalationInterval;
    const lastEscalation = input.time_since_last_escalation ?? 0;
    const remainingWait = Math.max(0, deferSeconds - lastEscalation);

    const deferUntil = new Date(Date.now() + remainingWait * 1000);
    return deferUntil.toISOString();
  }

  private buildAppliedPolicy(input: EscalationAgentInput): AppliedPolicy {
    return {
      policy_id: input.policy_id ?? this.config.escalation.defaultPolicyId ?? 'default',
      policy_name: 'Default Escalation Policy',
      policy_version: '1.0.0',
      triggered_level: input.current_escalation_level,
      max_level: this.config.escalation.maxEscalationLevel
    };
  }

  private generateReason(
    decision: EscalationDecision,
    thresholds: ThresholdEvaluation[],
    timeFactors: TimeFactor[],
    input: EscalationAgentInput
  ): string {
    const breachedThresholds = thresholds
      .filter(t => t.breached && t.weight > 0)
      .map(t => t.name);

    switch (decision) {
      case 'escalate':
        if (breachedThresholds.length > 0) {
          return `Escalation triggered due to: ${breachedThresholds.join(', ')}. Signal source: ${input.signal_source}.`;
        }
        return `Escalation triggered based on overall evaluation score. Signal source: ${input.signal_source}.`;

      case 'deescalate':
        return `Deescalation appropriate. Incident is in progress and no active escalation triggers.`;

      case 'maintain':
        return `No escalation change required. Current severity ${input.current_severity} at level ${input.current_escalation_level} is appropriate.`;

      case 'defer':
        return `Escalation evaluation deferred. Minimum interval (${this.config.escalation.minEscalationInterval}s) not met since last escalation.`;

      default:
        return `Decision: ${decision}`;
    }
  }
}

// ============================================================================
// RESULT TYPE
// ============================================================================

export interface EscalationResult {
  output: EscalationAgentOutput;
  confidence: number;
  confidenceFactors: ConfidenceFactor[];
  processingTimeMs: number;
}
