/**
 * ReasoningBank Controller - Pattern Storage and Retrieval
 *
 * Manages reasoning patterns with embeddings for semantic similarity search.
 * Integrates with ReasoningBank WASM for high-performance pattern matching.
 *
 * Pattern Structure:
 * - taskType: Type of task (e.g., "code_review", "data_analysis")
 * - approach: Description of the reasoning approach used
 * - successRate: Success rate of this pattern (0-1)
 * - embedding: Vector embedding of the pattern for similarity search
 * - metadata: Additional contextual information
 */

// Database type from db-fallback
type Database = any;
import { EmbeddingService } from './EmbeddingService.js';

export interface ReasoningPattern {
  id?: number;
  taskType: string;
  approach: string;
  successRate: number;
  embedding?: Float32Array;
  uses?: number;
  avgReward?: number;
  tags?: string[];
  metadata?: Record<string, any>;
  createdAt?: number;
  similarity?: number; // Cosine similarity score (for search results)
}

export interface PatternSearchQuery {
  taskEmbedding: Float32Array;
  k?: number;
  threshold?: number;
  filters?: {
    taskType?: string;
    minSuccessRate?: number;
    tags?: string[];
  };
}

export interface PatternStats {
  totalPatterns: number;
  avgSuccessRate: number;
  avgUses: number;
  topTaskTypes: Array<{ taskType: string; count: number }>;
  recentPatterns: number;
  highPerformingPatterns: number;
}

export class ReasoningBank {
  private db: Database;
  private embedder: EmbeddingService;
  private cache: Map<string, any>;

  constructor(db: Database, embedder: EmbeddingService) {
    this.db = db;
    this.embedder = embedder;
    this.cache = new Map();
    this.initializeSchema();
  }

  /**
   * Initialize reasoning patterns schema
   */
  private initializeSchema(): void {
    // Create patterns table
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS reasoning_patterns (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER DEFAULT (strftime('%s', 'now')),
        task_type TEXT NOT NULL,
        approach TEXT NOT NULL,
        success_rate REAL NOT NULL DEFAULT 0.0,
        uses INTEGER DEFAULT 0,
        avg_reward REAL DEFAULT 0.0,
        tags TEXT,
        metadata TEXT
      );

      CREATE INDEX IF NOT EXISTS idx_patterns_task_type ON reasoning_patterns(task_type);
      CREATE INDEX IF NOT EXISTS idx_patterns_success_rate ON reasoning_patterns(success_rate);
      CREATE INDEX IF NOT EXISTS idx_patterns_uses ON reasoning_patterns(uses);
    `);

    // Create pattern embeddings table
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS pattern_embeddings (
        pattern_id INTEGER PRIMARY KEY,
        embedding BLOB NOT NULL,
        FOREIGN KEY (pattern_id) REFERENCES reasoning_patterns(id) ON DELETE CASCADE
      );
    `);
  }

  /**
   * Store a reasoning pattern with embedding
   */
  async storePattern(pattern: ReasoningPattern): Promise<number> {
    // Generate embedding from approach text
    const embedding = await this.embedder.embed(
      `${pattern.taskType}: ${pattern.approach}`
    );

    // Insert pattern
    const stmt = this.db.prepare(`
      INSERT INTO reasoning_patterns (
        task_type, approach, success_rate, uses, avg_reward, tags, metadata
      ) VALUES (?, ?, ?, ?, ?, ?, ?)
    `);

    const result = stmt.run(
      pattern.taskType,
      pattern.approach,
      pattern.successRate,
      pattern.uses || 0,
      pattern.avgReward || 0.0,
      pattern.tags ? JSON.stringify(pattern.tags) : null,
      pattern.metadata ? JSON.stringify(pattern.metadata) : null
    );

    const patternId = result.lastInsertRowid as number;

    // Store embedding
    this.storePatternEmbedding(patternId, embedding);

    // Invalidate cache
    this.cache.clear();

    return patternId;
  }

  /**
   * Store pattern embedding
   */
  private storePatternEmbedding(patternId: number, embedding: Float32Array): void {
    const blob = Buffer.from(embedding.buffer);

    const stmt = this.db.prepare(`
      INSERT OR REPLACE INTO pattern_embeddings (pattern_id, embedding)
      VALUES (?, ?)
    `);

    stmt.run(patternId, blob);
  }

  /**
   * Search patterns by semantic similarity
   */
  async searchPatterns(query: PatternSearchQuery): Promise<ReasoningPattern[]> {
    const k = query.k || 10;
    const threshold = query.threshold || 0.0;

    // Build WHERE clause for filters
    const conditions: string[] = [];
    const params: any[] = [];

    if (query.filters?.taskType) {
      conditions.push('rp.task_type = ?');
      params.push(query.filters.taskType);
    }

    if (query.filters?.minSuccessRate !== undefined) {
      conditions.push('rp.success_rate >= ?');
      params.push(query.filters.minSuccessRate);
    }

    if (query.filters?.tags && query.filters.tags.length > 0) {
      // Check if any of the tags match
      const tagConditions = query.filters.tags.map(() => 'rp.tags LIKE ?').join(' OR ');
      conditions.push(`(${tagConditions})`);
      query.filters.tags.forEach(tag => {
        params.push(`%"${tag}"%`);
      });
    }

    const whereClause = conditions.length > 0
      ? `WHERE ${conditions.join(' AND ')}`
      : '';

    // Retrieve all candidate patterns
    const stmt = this.db.prepare(`
      SELECT
        rp.id,
        rp.ts,
        rp.task_type,
        rp.approach,
        rp.success_rate,
        rp.uses,
        rp.avg_reward,
        rp.tags,
        rp.metadata,
        pe.embedding
      FROM reasoning_patterns rp
      JOIN pattern_embeddings pe ON rp.id = pe.pattern_id
      ${whereClause}
    `);

    const rows = stmt.all(...params) as any[];

    // Calculate similarities
    const candidates = rows.map(row => {
      const embedding = new Float32Array(
        (row.embedding as Buffer).buffer,
        (row.embedding as Buffer).byteOffset,
        (row.embedding as Buffer).byteLength / 4
      );

      const similarity = this.cosineSimilarity(query.taskEmbedding, embedding);

      return {
        id: row.id,
        taskType: row.task_type,
        approach: row.approach,
        successRate: row.success_rate,
        uses: row.uses,
        avgReward: row.avg_reward,
        tags: row.tags ? JSON.parse(row.tags) : [],
        metadata: row.metadata ? JSON.parse(row.metadata) : {},
        createdAt: row.ts,
        embedding,
        similarity,
      };
    });

    // Filter by threshold and sort by similarity
    const filtered = candidates
      .filter(c => c.similarity >= threshold)
      .sort((a, b) => b.similarity - a.similarity)
      .slice(0, k);

    return filtered;
  }

  /**
   * Get pattern statistics
   */
  getPatternStats(): PatternStats {
    // Check cache first
    const cacheKey = 'pattern_stats';
    if (this.cache.has(cacheKey)) {
      return this.cache.get(cacheKey);
    }

    // Total patterns
    const totalRow = this.db.prepare(
      'SELECT COUNT(*) as count FROM reasoning_patterns'
    ).get() as any;

    // Average success rate and uses
    const avgRow = this.db.prepare(`
      SELECT
        AVG(success_rate) as avg_success_rate,
        AVG(uses) as avg_uses
      FROM reasoning_patterns
    `).get() as any;

    // Top task types
    const topTaskTypes = this.db.prepare(`
      SELECT
        task_type,
        COUNT(*) as count
      FROM reasoning_patterns
      GROUP BY task_type
      ORDER BY count DESC
      LIMIT 10
    `).all() as any[];

    // Recent patterns (last 7 days)
    const recentRow = this.db.prepare(`
      SELECT COUNT(*) as count
      FROM reasoning_patterns
      WHERE ts >= strftime('%s', 'now', '-7 days')
    `).get() as any;

    // High performing patterns (success_rate >= 0.8)
    const highPerfRow = this.db.prepare(`
      SELECT COUNT(*) as count
      FROM reasoning_patterns
      WHERE success_rate >= 0.8
    `).get() as any;

    const stats: PatternStats = {
      totalPatterns: totalRow.count,
      avgSuccessRate: avgRow.avg_success_rate || 0,
      avgUses: avgRow.avg_uses || 0,
      topTaskTypes: topTaskTypes.map(row => ({
        taskType: row.task_type,
        count: row.count,
      })),
      recentPatterns: recentRow.count,
      highPerformingPatterns: highPerfRow.count,
    };

    // Cache for 5 minutes
    this.cache.set(cacheKey, stats);
    setTimeout(() => this.cache.delete(cacheKey), 5 * 60 * 1000);

    return stats;
  }

  /**
   * Update pattern statistics after use
   */
  updatePatternStats(
    patternId: number,
    success: boolean,
    reward: number
  ): void {
    const stmt = this.db.prepare(`
      UPDATE reasoning_patterns
      SET
        uses = uses + 1,
        success_rate = (success_rate * uses + ?) / (uses + 1),
        avg_reward = (avg_reward * uses + ?) / (uses + 1)
      WHERE id = ?
    `);

    stmt.run(success ? 1 : 0, reward, patternId);

    // Invalidate cache
    this.cache.clear();
  }

  /**
   * Get pattern by ID
   */
  getPattern(patternId: number): ReasoningPattern | null {
    const stmt = this.db.prepare(`
      SELECT
        rp.id,
        rp.ts,
        rp.task_type,
        rp.approach,
        rp.success_rate,
        rp.uses,
        rp.avg_reward,
        rp.tags,
        rp.metadata,
        pe.embedding
      FROM reasoning_patterns rp
      LEFT JOIN pattern_embeddings pe ON rp.id = pe.pattern_id
      WHERE rp.id = ?
    `);

    const row = stmt.get(patternId) as any;
    if (!row) return null;

    return {
      id: row.id,
      taskType: row.task_type,
      approach: row.approach,
      successRate: row.success_rate,
      uses: row.uses,
      avgReward: row.avg_reward,
      tags: row.tags ? JSON.parse(row.tags) : [],
      metadata: row.metadata ? JSON.parse(row.metadata) : {},
      createdAt: row.ts,
      embedding: row.embedding
        ? new Float32Array(
            row.embedding.buffer,
            row.embedding.byteOffset,
            row.embedding.byteLength / 4
          )
        : undefined,
    };
  }

  /**
   * Delete pattern by ID
   */
  deletePattern(patternId: number): boolean {
    const stmt = this.db.prepare('DELETE FROM reasoning_patterns WHERE id = ?');
    const result = stmt.run(patternId);

    // Invalidate cache
    this.cache.clear();

    return result.changes > 0;
  }

  /**
   * Clear query cache
   */
  clearCache(): void {
    this.cache.clear();
  }

  /**
   * Calculate cosine similarity between two vectors
   */
  private cosineSimilarity(a: Float32Array, b: Float32Array): number {
    if (a.length !== b.length) {
      throw new Error('Vectors must have same length');
    }

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    const denom = Math.sqrt(normA) * Math.sqrt(normB);
    return denom === 0 ? 0 : dotProduct / denom;
  }
}
