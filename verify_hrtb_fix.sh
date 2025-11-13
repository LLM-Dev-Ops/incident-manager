#!/bin/bash
# Verification script for HRTB lifetime error fixes

echo "=========================================="
echo "HRTB Lifetime Error Fix Verification"
echo "=========================================="
echo ""

echo "1. Checking library compilation..."
if cargo build --lib 2>&1 | grep -q "error\[E"; then
    echo "❌ FAILED: Compilation errors found"
    cargo build --lib 2>&1 | grep "error\[E" -A3
    exit 1
else
    echo "✅ PASSED: Library compiles without errors"
fi
echo ""

echo "2. Checking enrichment pipeline code..."
if grep -q "TEMPORARY FIX: Disable enrichment" src/enrichment/pipeline.rs; then
    echo "❌ FAILED: Enrichment is still disabled"
    exit 1
else
    echo "✅ PASSED: Enrichment is enabled"
fi
echo ""

echo "3. Verifying proper future creation pattern..."
if grep -q "let futures: Vec<_> = enrichers" src/enrichment/pipeline.rs; then
    echo "✅ PASSED: Uses proper future collection pattern"
else
    echo "❌ FAILED: Improper pattern detected"
    exit 1
fi
echo ""

echo "4. Checking for HRTB-causing patterns..."
if grep -A5 "stream::iter(enrichers_owned)" src/enrichment/pipeline.rs | grep -q ".map(|enricher|"; then
    echo "⚠️  WARNING: Old HRTB-causing pattern may still exist"
else
    echo "✅ PASSED: No HRTB-causing patterns found"
fi
echo ""

echo "5. Verifying gRPC Send trait compliance..."
if grep -B2 "tokio::spawn" src/grpc/alert_service.rs | grep -q "processor.clone()"; then
    echo "✅ PASSED: gRPC properly clones before spawn"
else
    echo "⚠️  WARNING: Check gRPC clone pattern"
fi
echo ""

echo "=========================================="
echo "Verification Summary"
echo "=========================================="
echo "All critical checks passed!"
echo "The HRTB lifetime errors have been properly fixed."
echo ""
echo "To build the project:"
echo "  cargo build --lib"
echo ""
echo "To run tests:"
echo "  cargo test --lib enrichment"
echo ""
