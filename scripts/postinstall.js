#!/usr/bin/env node

/**
 * Post-installation script
 * Display helpful information after installation
 */

const fs = require('fs');
const path = require('path');

console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('  LLM Incident Manager - Enterprise-Grade Incident Management');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
console.log('📚 Quick Start:');
console.log('');
console.log('  1. Build the project:');
console.log('     npm run build');
console.log('');
console.log('  2. Start the server:');
console.log('     npm start');
console.log('');
console.log('  3. Access the GraphQL Playground:');
console.log('     http://localhost:8080/graphql/playground');
console.log('');
console.log('  4. Check health:');
console.log('     npm run health');
console.log('');
console.log('  5. View metrics:');
console.log('     npm run metrics');
console.log('');
console.log('📖 Documentation:');
console.log('  https://github.com/globalbusinessadvisors/llm-incident-manager');
console.log('');
console.log('🔧 Prerequisites:');
console.log('  - Rust 1.75+');
console.log('  - PostgreSQL 14+ (optional)');
console.log('  - Redis (optional)');
console.log('');
console.log('💡 Environment Variables:');
console.log('  DATABASE_URL     - PostgreSQL connection string');
console.log('  REDIS_URL        - Redis connection string');
console.log('  API_PORT         - REST API port (default: 8080)');
console.log('  GRPC_PORT        - gRPC port (default: 50051)');
console.log('  METRICS_PORT     - Prometheus metrics port (default: 9090)');
console.log('');
console.log('═══════════════════════════════════════════════════════════════');
console.log('');
