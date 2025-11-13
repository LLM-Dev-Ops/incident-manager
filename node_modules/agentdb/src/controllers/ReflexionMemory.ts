/**
 * ReflexionMemory - Episodic Replay Memory System
 *
 * Implements reflexion-style episodic replay for agent self-improvement.
 * Stores self-critiques and outcomes, retrieves relevant past experiences.
 *
 * Based on: "Reflexion: Language Agents with Verbal Reinforcement Learning"
 * https://arxiv.org/abs/2303.11366
 */

// Database type from db-fallback
type Database = any;
import { EmbeddingService } from './EmbeddingService.js';

export interface Episode {
  id?: number;
  ts?: number;
  sessionId: string;
  task: string;
  input?: string;
  output?: string;
  critique?: string;
  reward: number;
  success: boolean;
  latencyMs?: number;
  tokensUsed?: number;
  tags?: string[];
  metadata?: Record<string, any>;
}

export interface EpisodeWithEmbedding extends Episode {
  embedding?: Float32Array;
  similarity?: number;
}

export interface ReflexionQuery {
  task: string;
  currentState?: string;
  k?: number; // Top-k to retrieve
  minReward?: number;
  onlyFailures?: boolean;
  onlySuccesses?: boolean;
  timeWindowDays?: number;
}

export class ReflexionMemory {
  private db: Database;
  private embedder: EmbeddingService;

  constructor(db: Database, embedder: EmbeddingService) {
    this.db = db;
    this.embedder = embedder;
  }

  /**
   * Store a new episode with its critique and outcome
   */
  async storeEpisode(episode: Episode): Promise<number> {
    const stmt = this.db.prepare(`
      INSERT INTO episodes (
        session_id, task, input, output, critique, reward, success,
        latency_ms, tokens_used, tags, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    const tags = episode.tags ? JSON.stringify(episode.tags) : null;
    const metadata = episode.metadata ? JSON.stringify(episode.metadata) : null;

    const result = stmt.run(
      episode.sessionId,
      episode.task,
      episode.input || null,
      episode.output || null,
      episode.critique || null,
      episode.reward,
      episode.success ? 1 : 0,
      episode.latencyMs || null,
      episode.tokensUsed || null,
      tags,
      metadata
    );

    const episodeId = result.lastInsertRowid as number;

    // Generate and store embedding
    const text = this.buildEpisodeText(episode);
    const embedding = await this.embedder.embed(text);

    this.storeEmbedding(episodeId, embedding);

    return episodeId;
  }

  /**
   * Retrieve relevant past episodes for a new task attempt
   */
  async retrieveRelevant(query: ReflexionQuery): Promise<EpisodeWithEmbedding[]> {
    const {
      task,
      currentState = '',
      k = 5,
      minReward,
      onlyFailures = false,
      onlySuccesses = false,
      timeWindowDays
    } = query;

    // Generate query embedding
    const queryText = currentState ? `${task}\n${currentState}` : task;
    const queryEmbedding = await this.embedder.embed(queryText);

    // Build SQL filters
    const filters: string[] = [];
    const params: any[] = [];

    if (minReward !== undefined) {
      filters.push('e.reward >= ?');
      params.push(minReward);
    }

    if (onlyFailures) {
      filters.push('e.success = 0');
    }

    if (onlySuccesses) {
      filters.push('e.success = 1');
    }

    if (timeWindowDays) {
      filters.push('e.ts > strftime("%s", "now") - ?');
      params.push(timeWindowDays * 86400);
    }

    const whereClause = filters.length > 0 ? `WHERE ${filters.join(' AND ')}` : '';

    // Retrieve all candidates
    const stmt = this.db.prepare(`
      SELECT
        e.*,
        ee.embedding
      FROM episodes e
      JOIN episode_embeddings ee ON e.id = ee.episode_id
      ${whereClause}
      ORDER BY e.reward DESC
    `);

    const rows = stmt.all(...params) as any[];

    // Calculate similarities
    const episodes: EpisodeWithEmbedding[] = rows.map(row => {
      const embedding = this.deserializeEmbedding(row.embedding);
      const similarity = this.cosineSimilarity(queryEmbedding, embedding);

      return {
        id: row.id,
        ts: row.ts,
        sessionId: row.session_id,
        task: row.task,
        input: row.input,
        output: row.output,
        critique: row.critique,
        reward: row.reward,
        success: row.success === 1,
        latencyMs: row.latency_ms,
        tokensUsed: row.tokens_used,
        tags: row.tags ? JSON.parse(row.tags) : undefined,
        metadata: row.metadata ? JSON.parse(row.metadata) : undefined,
        embedding,
        similarity
      };
    });

    // Sort by similarity and return top-k
    episodes.sort((a, b) => (b.similarity || 0) - (a.similarity || 0));
    return episodes.slice(0, k);
  }

  /**
   * Get statistics for a task
   */
  getTaskStats(task: string, timeWindowDays?: number): {
    totalAttempts: number;
    successRate: number;
    avgReward: number;
    avgLatency: number;
    improvementTrend: number;
  } {
    const windowFilter = timeWindowDays
      ? `AND ts > strftime('%s', 'now') - ${timeWindowDays * 86400}`
      : '';

    const stmt = this.db.prepare(`
      SELECT
        COUNT(*) as total,
        AVG(CASE WHEN success = 1 THEN 1.0 ELSE 0.0 END) as success_rate,
        AVG(reward) as avg_reward,
        AVG(latency_ms) as avg_latency
      FROM episodes
      WHERE task = ? ${windowFilter}
    `);

    const stats = stmt.get(task) as any;

    // Calculate improvement trend (recent vs older)
    const trendStmt = this.db.prepare(`
      SELECT
        AVG(CASE
          WHEN ts > strftime('%s', 'now') - ${7 * 86400} THEN reward
        END) as recent_reward,
        AVG(CASE
          WHEN ts <= strftime('%s', 'now') - ${7 * 86400} THEN reward
        END) as older_reward
      FROM episodes
      WHERE task = ? ${windowFilter}
    `);

    const trend = trendStmt.get(task) as any;
    const improvementTrend = trend.recent_reward && trend.older_reward
      ? (trend.recent_reward - trend.older_reward) / trend.older_reward
      : 0;

    return {
      totalAttempts: stats.total || 0,
      successRate: stats.success_rate || 0,
      avgReward: stats.avg_reward || 0,
      avgLatency: stats.avg_latency || 0,
      improvementTrend
    };
  }

  /**
   * Build critique summary from similar failed episodes
   */
  async getCritiqueSummary(query: ReflexionQuery): Promise<string> {
    const failures = await this.retrieveRelevant({
      ...query,
      onlyFailures: true,
      k: 3
    });

    if (failures.length === 0) {
      return 'No prior failures found for this task.';
    }

    const critiques = failures
      .filter(ep => ep.critique)
      .map((ep, i) => `${i + 1}. ${ep.critique} (reward: ${ep.reward.toFixed(2)})`)
      .join('\n');

    return `Prior failures and lessons learned:\n${critiques}`;
  }

  /**
   * Get successful strategies for a task
   */
  async getSuccessStrategies(query: ReflexionQuery): Promise<string> {
    const successes = await this.retrieveRelevant({
      ...query,
      onlySuccesses: true,
      minReward: 0.7,
      k: 3
    });

    if (successes.length === 0) {
      return 'No successful strategies found for this task.';
    }

    const strategies = successes
      .map((ep, i) => {
        const approach = ep.output?.substring(0, 200) || 'No output recorded';
        return `${i + 1}. Approach (reward ${ep.reward.toFixed(2)}): ${approach}...`;
      })
      .join('\n');

    return `Successful strategies:\n${strategies}`;
  }

  /**
   * Prune low-quality episodes based on TTL and quality threshold
   */
  pruneEpisodes(config: {
    minReward?: number;
    maxAgeDays?: number;
    keepMinPerTask?: number;
  }): number {
    const { minReward = 0.3, maxAgeDays = 30, keepMinPerTask = 5 } = config;

    // Keep high-reward episodes and minimum per task
    const stmt = this.db.prepare(`
      DELETE FROM episodes
      WHERE id IN (
        SELECT id FROM (
          SELECT
            id,
            reward,
            ts,
            ROW_NUMBER() OVER (PARTITION BY task ORDER BY reward DESC) as rank
          FROM episodes
          WHERE reward < ?
            AND ts < strftime('%s', 'now') - ?
        ) WHERE rank > ?
      )
    `);

    const result = stmt.run(minReward, maxAgeDays * 86400, keepMinPerTask);
    return result.changes;
  }

  // ========================================================================
  // Private Helper Methods
  // ========================================================================

  private buildEpisodeText(episode: Episode): string {
    const parts = [episode.task];
    if (episode.critique) parts.push(episode.critique);
    if (episode.output) parts.push(episode.output);
    return parts.join('\n');
  }

  private storeEmbedding(episodeId: number, embedding: Float32Array): void {
    const stmt = this.db.prepare(`
      INSERT INTO episode_embeddings (episode_id, embedding)
      VALUES (?, ?)
    `);

    stmt.run(episodeId, this.serializeEmbedding(embedding));
  }

  private serializeEmbedding(embedding: Float32Array): Buffer {
    // Handle empty/null embeddings
    if (!embedding || !embedding.buffer) {
      return Buffer.alloc(0);
    }
    return Buffer.from(embedding.buffer);
  }

  private deserializeEmbedding(buffer: Buffer): Float32Array {
    return new Float32Array(buffer.buffer, buffer.byteOffset, buffer.length / 4);
  }

  private cosineSimilarity(a: Float32Array, b: Float32Array): number {
    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }
}
