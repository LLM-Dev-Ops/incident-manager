/**
 * CausalMemoryGraph - Causal Reasoning over Agent Memories
 *
 * Implements intervention-based reasoning rather than correlation.
 * Stores p(y|do(x)) estimates and tracks causal uplift across episodes.
 *
 * Based on:
 * - Pearl's do-calculus and causal inference
 * - Uplift modeling from A/B testing
 * - Instrumental variable methods
 */

// Database type from db-fallback
type Database = any;

export interface CausalEdge {
  id?: number;
  fromMemoryId: number;
  fromMemoryType: 'episode' | 'skill' | 'note' | 'fact';
  toMemoryId: number;
  toMemoryType: 'episode' | 'skill' | 'note' | 'fact';

  // Metrics
  similarity: number;
  uplift?: number; // E[y|do(x)] - E[y]
  confidence: number;
  sampleSize?: number;

  // Evidence
  evidenceIds?: string[];
  experimentIds?: string[];
  confounderScore?: number;

  // Explanation
  mechanism?: string;
  metadata?: Record<string, any>;
}

export interface CausalExperiment {
  id?: number;
  name: string;
  hypothesis: string;
  treatmentId: number;
  treatmentType: string;
  controlId?: number;

  // Design
  startTime: number;
  endTime?: number;
  sampleSize: number;

  // Results
  treatmentMean?: number;
  controlMean?: number;
  uplift?: number;
  pValue?: number;
  confidenceIntervalLow?: number;
  confidenceIntervalHigh?: number;

  status: 'running' | 'completed' | 'failed';
  metadata?: Record<string, any>;
}

export interface CausalObservation {
  experimentId: number;
  episodeId: number;
  isTreatment: boolean;
  outcomeValue: number;
  outcomeType: 'reward' | 'success' | 'latency';
  context?: Record<string, any>;
}

export interface CausalQuery {
  interventionMemoryId: number;
  interventionMemoryType: string;
  outcomeMemoryId?: number;
  minConfidence?: number;
  minUplift?: number;
}

export class CausalMemoryGraph {
  private db: Database;

  constructor(db: Database) {
    this.db = db;
  }

  /**
   * Add a causal edge between memories
   */
  addCausalEdge(edge: CausalEdge): number {
    const stmt = this.db.prepare(`
      INSERT INTO causal_edges (
        from_memory_id, from_memory_type, to_memory_id, to_memory_type,
        similarity, uplift, confidence, sample_size,
        evidence_ids, confounder_score,
        mechanism, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const result = stmt.run(
      edge.fromMemoryId,
      edge.fromMemoryType,
      edge.toMemoryId,
      edge.toMemoryType,
      edge.similarity,
      edge.uplift || null,
      edge.confidence,
      edge.sampleSize || null,
      edge.evidenceIds ? JSON.stringify(edge.evidenceIds) : null,
      edge.confounderScore || null,
      edge.mechanism || null,
      edge.metadata ? JSON.stringify(edge.metadata) : null
    );

    return result.lastInsertRowid as number;
  }

  /**
   * Create a causal experiment (A/B test)
   */
  createExperiment(experiment: CausalExperiment): number {
    const stmt = this.db.prepare(`
      INSERT INTO causal_experiments (
        name, hypothesis, treatment_id, treatment_type, control_id,
        start_time, sample_size, status, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const result = stmt.run(
      experiment.name,
      experiment.hypothesis,
      experiment.treatmentId,
      experiment.treatmentType,
      experiment.controlId || null,
      experiment.startTime,
      experiment.sampleSize,
      experiment.status,
      experiment.metadata ? JSON.stringify(experiment.metadata) : null
    );

    return result.lastInsertRowid as number;
  }

  /**
   * Record an observation in an experiment
   */
  recordObservation(observation: CausalObservation): void {
    const stmt = this.db.prepare(`
      INSERT INTO causal_observations (
        experiment_id, episode_id, is_treatment, outcome_value, outcome_type, context
      ) VALUES (?, ?, ?, ?, ?, ?)
    `);

    stmt.run(
      observation.experimentId,
      observation.episodeId,
      observation.isTreatment ? 1 : 0,
      observation.outcomeValue,
      observation.outcomeType,
      observation.context ? JSON.stringify(observation.context) : null
    );

    // Update sample size
    this.db.prepare(`
      UPDATE causal_experiments
      SET sample_size = sample_size + 1
      WHERE id = ?
    `).run(observation.experimentId);
  }

  /**
   * Calculate uplift for an experiment
   */
  calculateUplift(experimentId: number): {
    uplift: number;
    pValue: number;
    confidenceInterval: [number, number];
  } {
    // Get treatment and control observations
    const observations = this.db.prepare(`
      SELECT is_treatment, outcome_value
      FROM causal_observations
      WHERE experiment_id = ?
    `).all(experimentId) as any[];

    const treatmentValues = observations
      .filter(o => o.is_treatment === 1)
      .map(o => o.outcome_value);

    const controlValues = observations
      .filter(o => o.is_treatment === 0)
      .map(o => o.outcome_value);

    if (treatmentValues.length === 0 || controlValues.length === 0) {
      return { uplift: 0, pValue: 1.0, confidenceInterval: [0, 0] };
    }

    // Calculate means
    const treatmentMean = this.mean(treatmentValues);
    const controlMean = this.mean(controlValues);
    const uplift = treatmentMean - controlMean;

    // Calculate standard errors
    const treatmentSE = this.standardError(treatmentValues);
    const controlSE = this.standardError(controlValues);
    const pooledSE = Math.sqrt(treatmentSE ** 2 + controlSE ** 2);

    // t-statistic and p-value (two-tailed)
    const tStat = uplift / pooledSE;
    const df = treatmentValues.length + controlValues.length - 2;
    const pValue = 2 * (1 - this.tCDF(Math.abs(tStat), df));

    // 95% confidence interval
    const tCritical = this.tInverse(0.025, df);
    const marginOfError = tCritical * pooledSE;
    const confidenceInterval: [number, number] = [
      uplift - marginOfError,
      uplift + marginOfError
    ];

    // Update experiment with results
    this.db.prepare(`
      UPDATE causal_experiments
      SET treatment_mean = ?,
          control_mean = ?,
          uplift = ?,
          p_value = ?,
          confidence_interval_low = ?,
          confidence_interval_high = ?,
          status = 'completed'
      WHERE id = ?
    `).run(
      treatmentMean,
      controlMean,
      uplift,
      pValue,
      confidenceInterval[0],
      confidenceInterval[1],
      experimentId
    );

    return { uplift, pValue, confidenceInterval };
  }

  /**
   * Query causal effects
   */
  queryCausalEffects(query: CausalQuery): CausalEdge[] {
    const {
      interventionMemoryId,
      interventionMemoryType,
      outcomeMemoryId,
      minConfidence = 0.5,
      minUplift = 0.0
    } = query;

    let sql = `
      SELECT * FROM causal_edges
      WHERE from_memory_id = ?
        AND from_memory_type = ?
        AND confidence >= ?
        AND ABS(uplift) >= ?
    `;

    const params: any[] = [
      interventionMemoryId,
      interventionMemoryType,
      minConfidence,
      minUplift
    ];

    if (outcomeMemoryId) {
      sql += ' AND to_memory_id = ?';
      params.push(outcomeMemoryId);
    }

    sql += ' ORDER BY ABS(uplift) * confidence DESC';

    const rows = this.db.prepare(sql).all(...params) as any[];

    return rows.map(row => this.rowToCausalEdge(row));
  }

  /**
   * Get causal chain (multi-hop reasoning)
   */
  getCausalChain(fromMemoryId: number, toMemoryId: number, maxDepth: number = 5): {
    path: number[];
    totalUplift: number;
    confidence: number;
  }[] {
    // Use recursive CTE from view
    const chains = this.db.prepare(`
      WITH RECURSIVE chain(from_id, to_id, depth, path, total_uplift, min_confidence) AS (
        SELECT
          from_memory_id,
          to_memory_id,
          1,
          from_memory_id || '->' || to_memory_id,
          uplift,
          confidence
        FROM causal_edges
        WHERE from_memory_id = ? AND confidence >= 0.5

        UNION ALL

        SELECT
          chain.from_id,
          ce.to_memory_id,
          chain.depth + 1,
          chain.path || '->' || ce.to_memory_id,
          chain.total_uplift + ce.uplift,
          MIN(chain.min_confidence, ce.confidence)
        FROM chain
        JOIN causal_edges ce ON chain.to_id = ce.from_memory_id
        WHERE chain.depth < ?
          AND ce.confidence >= 0.5
          AND chain.path NOT LIKE '%' || ce.to_memory_id || '%'
      )
      SELECT path, total_uplift, min_confidence
      FROM chain
      WHERE to_id = ?
      ORDER BY total_uplift DESC
      LIMIT 10
    `).all(fromMemoryId, maxDepth, toMemoryId) as any[];

    return chains.map(row => ({
      path: row.path.split('->').map(Number),
      totalUplift: row.total_uplift,
      confidence: row.min_confidence
    }));
  }

  /**
   * Calculate causal gain: E[outcome|do(treatment)] - E[outcome]
   */
  calculateCausalGain(treatmentId: number, outcomeType: 'reward' | 'success' | 'latency'): {
    causalGain: number;
    confidence: number;
    mechanism: string;
  } {
    // Get episodes where treatment was applied
    const withTreatment = this.db.prepare(`
      SELECT AVG(CASE WHEN ? = 'reward' THEN reward
                     WHEN ? = 'success' THEN success
                     WHEN ? = 'latency' THEN latency_ms
                END) as avg_outcome
      FROM episodes
      WHERE id IN (
        SELECT to_memory_id FROM causal_edges
        WHERE from_memory_id = ? AND confidence >= 0.6
      )
    `).get(outcomeType, outcomeType, outcomeType, treatmentId) as any;

    // Get baseline (no treatment)
    const baseline = this.db.prepare(`
      SELECT AVG(CASE WHEN ? = 'reward' THEN reward
                     WHEN ? = 'success' THEN success
                     WHEN ? = 'latency' THEN latency_ms
                END) as avg_outcome
      FROM episodes
      WHERE id NOT IN (
        SELECT to_memory_id FROM causal_edges
        WHERE from_memory_id = ?
      )
    `).get(outcomeType, outcomeType, outcomeType, treatmentId) as any;

    const causalGain = (withTreatment?.avg_outcome || 0) - (baseline?.avg_outcome || 0);

    // Get most confident edge for mechanism
    const edge = this.db.prepare(`
      SELECT mechanism, confidence
      FROM causal_edges
      WHERE from_memory_id = ?
      ORDER BY confidence DESC
      LIMIT 1
    `).get(treatmentId) as any;

    return {
      causalGain,
      confidence: edge?.confidence || 0,
      mechanism: edge?.mechanism || 'unknown'
    };
  }

  /**
   * Detect confounders using correlation analysis
   */
  detectConfounders(edgeId: number): {
    confounders: Array<{
      memoryId: number;
      correlationWithTreatment: number;
      correlationWithOutcome: number;
      confounderScore: number;
    }>;
  } {
    const edge = this.db.prepare('SELECT * FROM causal_edges WHERE id = ?').get(edgeId) as any;

    if (!edge) {
      return { confounders: [] };
    }

    // Find memories correlated with both treatment and outcome
    // This is a simplified version - production would use proper statistical tests
    const potentialConfounders = this.db.prepare(`
      SELECT DISTINCT e.id, e.task
      FROM episodes e
      WHERE e.id != ? AND e.id != ?
        AND e.session_id IN (
          SELECT session_id FROM episodes WHERE id = ?
          UNION
          SELECT session_id FROM episodes WHERE id = ?
        )
    `).all(edge.from_memory_id, edge.to_memory_id, edge.from_memory_id, edge.to_memory_id) as any[];

    const confounders = potentialConfounders.map((conf: any) => {
      // Calculate correlation scores (simplified)
      const treatmentCorr = this.calculateCorrelation(conf.id, edge.from_memory_id);
      const outcomeCorr = this.calculateCorrelation(conf.id, edge.to_memory_id);
      const confounderScore = Math.sqrt(treatmentCorr ** 2 * outcomeCorr ** 2);

      return {
        memoryId: conf.id,
        correlationWithTreatment: treatmentCorr,
        correlationWithOutcome: outcomeCorr,
        confounderScore
      };
    }).filter(c => c.confounderScore > 0.3);

    // Update edge with confounder score
    if (confounders.length > 0) {
      const maxConfounderScore = Math.max(...confounders.map(c => c.confounderScore));
      this.db.prepare(`
        UPDATE causal_edges
        SET confounder_score = ?
        WHERE id = ?
      `).run(maxConfounderScore, edgeId);
    }

    return { confounders };
  }

  // ========================================================================
  // Private Helper Methods
  // ========================================================================

  private rowToCausalEdge(row: any): CausalEdge {
    return {
      id: row.id,
      fromMemoryId: row.from_memory_id,
      fromMemoryType: row.from_memory_type,
      toMemoryId: row.to_memory_id,
      toMemoryType: row.to_memory_type,
      similarity: row.similarity,
      uplift: row.uplift,
      confidence: row.confidence,
      sampleSize: row.sample_size,
      evidenceIds: row.evidence_ids ? JSON.parse(row.evidence_ids) : undefined,
      experimentIds: row.experiment_ids ? JSON.parse(row.experiment_ids) : undefined,
      confounderScore: row.confounder_score,
      mechanism: row.mechanism,
      metadata: row.metadata ? JSON.parse(row.metadata) : undefined
    };
  }

  private mean(values: number[]): number {
    return values.reduce((a, b) => a + b, 0) / values.length;
  }

  private variance(values: number[]): number {
    const avg = this.mean(values);
    return values.reduce((sum, val) => sum + (val - avg) ** 2, 0) / values.length;
  }

  private standardError(values: number[]): number {
    return Math.sqrt(this.variance(values) / values.length);
  }

  private tCDF(t: number, df: number): number {
    // Simplified t-distribution CDF (use proper stats library in production)
    // This is an approximation
    return 0.5 + 0.5 * Math.sign(t) * (1 - Math.pow(1 + t * t / df, -df / 2));
  }

  private tInverse(p: number, df: number): number {
    // Simplified inverse t-distribution (use proper stats library)
    // Approximation for 95% CI
    return 1.96; // Standard normal approximation
  }

  private calculateCorrelation(id1: number, id2: number): number {
    // Simplified correlation calculation
    // In production, use proper correlation metrics
    const sharedSessions = this.db.prepare(`
      SELECT COUNT(DISTINCT e1.session_id) as shared
      FROM episodes e1
      JOIN episodes e2 ON e1.session_id = e2.session_id
      WHERE e1.id = ? AND e2.id = ?
    `).get(id1, id2) as any;

    return Math.min(sharedSessions?.shared || 0, 1.0);
  }
}
