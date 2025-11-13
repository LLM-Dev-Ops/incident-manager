#!/bin/bash

# Monitoring Setup for Flow Nexus Accounting System
# Sets up monitoring, alerts, and health checks

set -e

echo "📊 Setting up monitoring for Flow Nexus Accounting System..."

# Check environment
if [ -z "$SUPABASE_URL" ] || [ -z "$SUPABASE_SERVICE_ROLE_KEY" ]; then
    echo "❌ Required environment variables not set"
    exit 1
fi

BASE_URL="$SUPABASE_URL/functions/v1"

# Create monitoring database tables
echo "📁 Creating monitoring tables..."

psql "$DATABASE_URL" << 'EOF'
-- System health monitoring
CREATE TABLE IF NOT EXISTS system_health_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    service VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT,
    metadata JSONB DEFAULT '{}'
);

-- Transaction monitoring
CREATE TABLE IF NOT EXISTS transaction_monitoring (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    transaction_count INTEGER,
    avg_processing_time_ms INTEGER,
    error_count INTEGER,
    peak_tps INTEGER,
    metadata JSONB DEFAULT '{}'
);

-- Balance integrity monitoring
CREATE TABLE IF NOT EXISTS balance_integrity_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    is_balanced BOOLEAN,
    total_debits DECIMAL(15,4),
    total_credits DECIMAL(15,4),
    difference DECIMAL(15,4),
    unbalanced_count INTEGER
);

-- Performance metrics
CREATE TABLE IF NOT EXISTS performance_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    timestamp TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    metric_name VARCHAR(100),
    metric_value DECIMAL(15,4),
    unit VARCHAR(20),
    tags JSONB DEFAULT '{}'
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_system_health_timestamp ON system_health_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_transaction_monitoring_timestamp ON transaction_monitoring(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_balance_integrity_timestamp ON balance_integrity_log(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_performance_metrics_timestamp ON performance_metrics(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_performance_metrics_name ON performance_metrics(metric_name);

-- Create monitoring functions
CREATE OR REPLACE FUNCTION log_system_health(
    p_service VARCHAR(50),
    p_status VARCHAR(20),
    p_response_time_ms INTEGER DEFAULT NULL,
    p_error_message TEXT DEFAULT NULL,
    p_metadata JSONB DEFAULT '{}'
) RETURNS UUID AS $$
DECLARE
    health_id UUID;
BEGIN
    INSERT INTO system_health_log (service, status, response_time_ms, error_message, metadata)
    VALUES (p_service, p_status, p_response_time_ms, p_error_message, p_metadata)
    RETURNING id INTO health_id;
    
    RETURN health_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION log_transaction_metrics(
    p_transaction_count INTEGER,
    p_avg_processing_time_ms INTEGER,
    p_error_count INTEGER,
    p_peak_tps INTEGER,
    p_metadata JSONB DEFAULT '{}'
) RETURNS UUID AS $$
DECLARE
    metric_id UUID;
BEGIN
    INSERT INTO transaction_monitoring (transaction_count, avg_processing_time_ms, error_count, peak_tps, metadata)
    VALUES (p_transaction_count, p_avg_processing_time_ms, p_error_count, p_peak_tps, p_metadata)
    RETURNING id INTO metric_id;
    
    RETURN metric_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION log_balance_integrity(
    p_is_balanced BOOLEAN,
    p_total_debits DECIMAL(15,4),
    p_total_credits DECIMAL(15,4),
    p_difference DECIMAL(15,4),
    p_unbalanced_count INTEGER
) RETURNS UUID AS $$
DECLARE
    integrity_id UUID;
BEGIN
    INSERT INTO balance_integrity_log (is_balanced, total_debits, total_credits, difference, unbalanced_count)
    VALUES (p_is_balanced, p_total_debits, p_total_credits, p_difference, p_unbalanced_count)
    RETURNING id INTO integrity_id;
    
    RETURN integrity_id;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION log_performance_metric(
    p_metric_name VARCHAR(100),
    p_metric_value DECIMAL(15,4),
    p_unit VARCHAR(20),
    p_tags JSONB DEFAULT '{}'
) RETURNS UUID AS $$
DECLARE
    metric_id UUID;
BEGIN
    INSERT INTO performance_metrics (metric_name, metric_value, unit, tags)
    VALUES (p_metric_name, p_metric_value, p_unit, p_tags)
    RETURNING id INTO metric_id;
    
    RETURN metric_id;
END;
$$ LANGUAGE plpgsql;

-- Schedule periodic health checks (every 5 minutes)
SELECT cron.schedule(
    'accounting-health-check',
    '*/5 * * * *',
    $$
    DO $$
    DECLARE
        start_time TIMESTAMP;
        end_time TIMESTAMP;
        response_time INTEGER;
        trial_balance RECORD;
        transaction_count INTEGER;
        avg_time INTEGER;
        error_count INTEGER;
    BEGIN
        start_time := NOW();
        
        -- Check trial balance
        SELECT 
            SUM(debit_amount) = SUM(credit_amount) as is_balanced,
            SUM(debit_amount) as total_debits,
            SUM(credit_amount) as total_credits,
            ABS(SUM(debit_amount) - SUM(credit_amount)) as difference
        INTO trial_balance
        FROM ledger_entries le
        JOIN transactions t ON le.transaction_id = t.id
        WHERE t.status = 'completed';
        
        -- Log balance integrity
        PERFORM log_balance_integrity(
            trial_balance.is_balanced,
            trial_balance.total_debits,
            trial_balance.total_credits,
            trial_balance.difference,
            CASE WHEN trial_balance.is_balanced THEN 0 ELSE 1 END
        );
        
        -- Check transaction metrics (last hour)
        SELECT 
            COUNT(*) as tx_count,
            AVG(EXTRACT(EPOCH FROM (updated_at - created_at)) * 1000) as avg_ms,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as err_count
        INTO transaction_count, avg_time, error_count
        FROM transactions
        WHERE created_at >= NOW() - INTERVAL '1 hour';
        
        -- Log transaction metrics
        PERFORM log_transaction_metrics(
            transaction_count,
            avg_time::INTEGER,
            error_count,
            GREATEST(1, transaction_count / 3600) -- TPS for last hour
        );
        
        end_time := NOW();
        response_time := EXTRACT(EPOCH FROM (end_time - start_time)) * 1000;
        
        -- Log overall health
        PERFORM log_system_health(
            'accounting-system',
            CASE WHEN trial_balance.is_balanced AND error_count < 10 THEN 'healthy' ELSE 'warning' END,
            response_time,
            CASE WHEN NOT trial_balance.is_balanced THEN 'Trial balance not balanced' 
                 WHEN error_count >= 10 THEN 'High error rate detected'
                 ELSE NULL END
        );
        
    EXCEPTION WHEN OTHERS THEN
        PERFORM log_system_health(
            'accounting-system',
            'error',
            NULL,
            SQLERRM
        );
    END $$;
    $$
);

-- Schedule daily reporting (every day at 1 AM)
SELECT cron.schedule(
    'daily-accounting-report',
    '0 1 * * *',
    $$
    DO $$
    DECLARE
        report_data JSONB;
        yesterday DATE;
    BEGIN
        yesterday := CURRENT_DATE - INTERVAL '1 day';
        
        -- Generate daily summary
        SELECT jsonb_build_object(
            'date', yesterday,
            'total_transactions', COUNT(*),
            'total_volume', SUM(
                CASE WHEN le.debit_amount > 0 THEN le.debit_amount ELSE le.credit_amount END
            ),
            'unique_users', COUNT(DISTINCT t.user_id),
            'avg_processing_time', AVG(EXTRACT(EPOCH FROM (t.updated_at - t.created_at)) * 1000),
            'error_rate', 
                COUNT(CASE WHEN t.status = 'failed' THEN 1 END)::FLOAT / NULLIF(COUNT(*), 0) * 100
        ) INTO report_data
        FROM transactions t
        JOIN ledger_entries le ON t.id = le.transaction_id
        WHERE DATE(t.created_at) = yesterday;
        
        -- Log daily report
        PERFORM log_performance_metric(
            'daily_summary',
            (report_data->>'total_transactions')::DECIMAL,
            'count',
            report_data
        );
        
    END $$;
    $$
);

EOF

echo "✅ Monitoring tables and functions created"

# Create monitoring dashboard views
echo "📊 Creating monitoring dashboard views..."

psql "$DATABASE_URL" << 'EOF'
-- Real-time system health view
CREATE OR REPLACE VIEW system_health_dashboard AS
SELECT 
    service,
    status,
    timestamp,
    response_time_ms,
    error_message,
    LAG(timestamp) OVER (PARTITION BY service ORDER BY timestamp) as previous_check,
    timestamp - LAG(timestamp) OVER (PARTITION BY service ORDER BY timestamp) as uptime_since_last
FROM system_health_log
WHERE timestamp >= NOW() - INTERVAL '24 hours'
ORDER BY timestamp DESC;

-- Performance trends view
CREATE OR REPLACE VIEW performance_trends AS
SELECT 
    DATE_TRUNC('hour', timestamp) as hour,
    AVG(transaction_count) as avg_transactions_per_hour,
    AVG(avg_processing_time_ms) as avg_processing_time,
    AVG(error_count) as avg_errors_per_hour,
    MAX(peak_tps) as peak_tps
FROM transaction_monitoring
WHERE timestamp >= NOW() - INTERVAL '7 days'
GROUP BY DATE_TRUNC('hour', timestamp)
ORDER BY hour DESC;

-- Balance integrity status
CREATE OR REPLACE VIEW balance_integrity_status AS
SELECT 
    DATE_TRUNC('day', timestamp) as day,
    COUNT(*) as checks_performed,
    COUNT(CASE WHEN is_balanced THEN 1 END) as balanced_checks,
    COUNT(CASE WHEN NOT is_balanced THEN 1 END) as unbalanced_checks,
    AVG(ABS(difference)) as avg_difference,
    MAX(ABS(difference)) as max_difference
FROM balance_integrity_log
WHERE timestamp >= NOW() - INTERVAL '30 days'
GROUP BY DATE_TRUNC('day', timestamp)
ORDER BY day DESC;

-- Top metrics view
CREATE OR REPLACE VIEW top_metrics AS
SELECT 
    metric_name,
    AVG(metric_value) as avg_value,
    MIN(metric_value) as min_value,
    MAX(metric_value) as max_value,
    COUNT(*) as sample_count,
    unit
FROM performance_metrics
WHERE timestamp >= NOW() - INTERVAL '24 hours'
GROUP BY metric_name, unit
ORDER BY avg_value DESC;

EOF

echo "✅ Dashboard views created"

# Create health check endpoints
echo "🏥 Creating health check endpoints..."

curl -X POST "$BASE_URL/realtime-coordinator?action=setup_channels" \
    -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY" \
    -H "Content-Type: application/json" \
    -d '{
        "monitoring": true,
        "alerts": true
    }' > /dev/null

# Create alerting functions
echo "🚨 Setting up alerting..."

psql "$DATABASE_URL" << 'EOF'
-- Alert configuration table
CREATE TABLE IF NOT EXISTS alert_config (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_name VARCHAR(100) UNIQUE NOT NULL,
    condition_sql TEXT NOT NULL,
    threshold_value DECIMAL(15,4),
    comparison_operator VARCHAR(10), -- '>', '<', '=', '>=', '<='
    severity VARCHAR(20) DEFAULT 'warning', -- 'info', 'warning', 'error', 'critical'
    notification_channel VARCHAR(50), -- 'email', 'slack', 'webhook'
    notification_config JSONB DEFAULT '{}',
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Alert history
CREATE TABLE IF NOT EXISTS alert_history (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    alert_name VARCHAR(100) NOT NULL,
    triggered_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    severity VARCHAR(20),
    message TEXT,
    metric_value DECIMAL(15,4),
    threshold_value DECIMAL(15,4),
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    acknowledged_by VARCHAR(100),
    resolved_at TIMESTAMP WITH TIME ZONE
);

-- Insert default alert configurations
INSERT INTO alert_config (alert_name, condition_sql, threshold_value, comparison_operator, severity, notification_channel) VALUES
('High Error Rate', 'SELECT AVG(error_count) FROM transaction_monitoring WHERE timestamp >= NOW() - INTERVAL ''1 hour''', 10, '>', 'warning', 'email'),
('Trial Balance Unbalanced', 'SELECT COUNT(*) FROM balance_integrity_log WHERE timestamp >= NOW() - INTERVAL ''1 hour'' AND NOT is_balanced', 0, '>', 'critical', 'email'),
('Slow Transaction Processing', 'SELECT AVG(avg_processing_time_ms) FROM transaction_monitoring WHERE timestamp >= NOW() - INTERVAL ''15 minutes''', 2000, '>', 'warning', 'email'),
('Low Transaction Volume', 'SELECT AVG(transaction_count) FROM transaction_monitoring WHERE timestamp >= NOW() - INTERVAL ''1 hour''', 1, '<', 'info', 'email'),
('System Health Down', 'SELECT COUNT(*) FROM system_health_log WHERE timestamp >= NOW() - INTERVAL ''10 minutes'' AND status != ''healthy''', 2, '>', 'critical', 'email')
ON CONFLICT (alert_name) DO NOTHING;

-- Alert evaluation function
CREATE OR REPLACE FUNCTION evaluate_alerts()
RETURNS TABLE(alert_name VARCHAR(100), triggered BOOLEAN, current_value DECIMAL(15,4), message TEXT) AS $$
DECLARE
    alert_record RECORD;
    result_value DECIMAL(15,4);
    is_triggered BOOLEAN;
    alert_message TEXT;
BEGIN
    FOR alert_record IN 
        SELECT * FROM alert_config WHERE is_active = true
    LOOP
        -- Execute the condition SQL
        EXECUTE alert_record.condition_sql INTO result_value;
        
        -- Check if alert should trigger
        CASE alert_record.comparison_operator
            WHEN '>' THEN is_triggered := result_value > alert_record.threshold_value;
            WHEN '<' THEN is_triggered := result_value < alert_record.threshold_value;
            WHEN '>=' THEN is_triggered := result_value >= alert_record.threshold_value;
            WHEN '<=' THEN is_triggered := result_value <= alert_record.threshold_value;
            WHEN '=' THEN is_triggered := result_value = alert_record.threshold_value;
            ELSE is_triggered := false;
        END CASE;
        
        -- Generate alert message
        alert_message := format('Alert: %s - Current value: %s %s threshold: %s', 
            alert_record.alert_name,
            result_value,
            alert_record.comparison_operator,
            alert_record.threshold_value
        );
        
        -- Log alert if triggered
        IF is_triggered THEN
            INSERT INTO alert_history (alert_name, severity, message, metric_value, threshold_value)
            VALUES (alert_record.alert_name, alert_record.severity, alert_message, result_value, alert_record.threshold_value);
        END IF;
        
        -- Return result
        RETURN QUERY SELECT alert_record.alert_name, is_triggered, result_value, alert_message;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Schedule alert evaluation every 5 minutes
SELECT cron.schedule(
    'alert-evaluation',
    '*/5 * * * *',
    'SELECT evaluate_alerts();'
);

EOF

echo "✅ Alerting system configured"

# Create monitoring script
echo "📝 Creating monitoring script..."

cat > scripts/monitor-accounting.sh << 'MONITOR_EOF'
#!/bin/bash

# Continuous monitoring script for Flow Nexus Accounting System

BASE_URL="${SUPABASE_URL}/functions/v1"
LOG_FILE="logs/accounting-monitor.log"

mkdir -p logs

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $1" | tee -a "$LOG_FILE"
}

check_health() {
    local service="$1"
    local endpoint="$2"
    
    start_time=$(date +%s%N)
    response=$(curl -s -w "%{http_code}" -o /tmp/response.json "$endpoint" \
        -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY")
    end_time=$(date +%s%N)
    
    response_time=$(( (end_time - start_time) / 1000000 ))
    http_code="${response: -3}"
    
    if [ "$http_code" = "200" ]; then
        log "✅ $service: OK (${response_time}ms)"
        return 0
    else
        log "❌ $service: FAILED (HTTP $http_code, ${response_time}ms)"
        cat /tmp/response.json >> "$LOG_FILE"
        return 1
    fi
}

main() {
    log "🔍 Starting accounting system health check..."
    
    # Check core services
    check_health "Transaction Processor" "$BASE_URL/transaction-processor"
    check_health "Balance Calculator" "$BASE_URL/balance-calculator"
    check_health "Reporting Analytics" "$BASE_URL/reporting-analytics"
    check_health "Realtime Coordinator" "$BASE_URL/realtime-coordinator?action=status"
    
    # Check trial balance
    response=$(curl -s "$BASE_URL/balance-calculator?operation=trial_balance" \
        -H "Authorization: Bearer $SUPABASE_SERVICE_ROLE_KEY")
    
    is_balanced=$(echo "$response" | jq -r '.trial_balance.is_balanced // false')
    
    if [ "$is_balanced" = "true" ]; then
        log "✅ Trial Balance: BALANCED"
    else
        total_debits=$(echo "$response" | jq -r '.trial_balance.total_debits // 0')
        total_credits=$(echo "$response" | jq -r '.trial_balance.total_credits // 0')
        log "❌ Trial Balance: UNBALANCED (Debits: $total_debits, Credits: $total_credits)"
    fi
    
    log "📊 Health check completed"
    echo ""
}

# Run continuously if --continuous flag is provided
if [ "$1" = "--continuous" ]; then
    log "🔄 Starting continuous monitoring (every 60 seconds)..."
    while true; do
        main
        sleep 60
    done
else
    main
fi
MONITOR_EOF

chmod +x scripts/monitor-accounting.sh

# Create Grafana/Prometheus compatible metrics endpoint
echo "📈 Creating metrics endpoint..."

cat > scripts/export-metrics.sh << 'METRICS_EOF'
#!/bin/bash

# Export metrics in Prometheus format

METRICS_FILE="metrics/accounting-metrics.prom"
mkdir -p metrics

cat > "$METRICS_FILE" << EOF
# HELP accounting_transactions_total Total number of transactions processed
# TYPE accounting_transactions_total counter
accounting_transactions_total $(psql "$DATABASE_URL" -t -c "SELECT COUNT(*) FROM transactions WHERE status = 'completed';" | tr -d ' ')

# HELP accounting_balance_total Current total balance across all accounts  
# TYPE accounting_balance_total gauge
accounting_balance_total $(psql "$DATABASE_URL" -t -c "SELECT COALESCE(SUM(balance), 0) FROM account_balances;" | tr -d ' ')

# HELP accounting_processing_time_ms Average transaction processing time in milliseconds
# TYPE accounting_processing_time_ms gauge
accounting_processing_time_ms $(psql "$DATABASE_URL" -t -c "SELECT COALESCE(AVG(avg_processing_time_ms), 0) FROM transaction_monitoring WHERE timestamp >= NOW() - INTERVAL '1 hour';" | tr -d ' ')

# HELP accounting_error_rate Percentage of failed transactions
# TYPE accounting_error_rate gauge
accounting_error_rate $(psql "$DATABASE_URL" -t -c "SELECT COALESCE(AVG(error_count), 0) FROM transaction_monitoring WHERE timestamp >= NOW() - INTERVAL '1 hour';" | tr -d ' ')

# HELP accounting_trial_balance_status Trial balance status (1 = balanced, 0 = unbalanced)
# TYPE accounting_trial_balance_status gauge
accounting_trial_balance_status $(psql "$DATABASE_URL" -t -c "SELECT CASE WHEN is_balanced THEN 1 ELSE 0 END FROM balance_integrity_log ORDER BY timestamp DESC LIMIT 1;" | tr -d ' ')

EOF

echo "Metrics exported to $METRICS_FILE"
METRICS_EOF

chmod +x scripts/export-metrics.sh

echo ""
echo "🎉 Monitoring Setup Complete!"
echo ""
echo "📊 Monitoring Features Installed:"
echo "   ✅ Health check monitoring"
echo "   ✅ Performance metrics tracking"
echo "   ✅ Balance integrity monitoring"
echo "   ✅ Automated alerting system"
echo "   ✅ Dashboard views"
echo "   ✅ Continuous monitoring scripts"
echo "   ✅ Prometheus metrics export"
echo ""
echo "🔧 Available Commands:"
echo "   Health Check: ./scripts/monitor-accounting.sh"
echo "   Continuous Monitor: ./scripts/monitor-accounting.sh --continuous"
echo "   Export Metrics: ./scripts/export-metrics.sh"
echo ""
echo "📈 Dashboard Queries:"
echo "   System Health: SELECT * FROM system_health_dashboard;"
echo "   Performance Trends: SELECT * FROM performance_trends;"
echo "   Balance Integrity: SELECT * FROM balance_integrity_status;"
echo "   Active Alerts: SELECT * FROM alert_history WHERE resolved_at IS NULL;"
echo ""
echo "🚨 Alert Configuration:"
echo "   View Alerts: SELECT * FROM alert_config;"
echo "   Alert History: SELECT * FROM alert_history ORDER BY triggered_at DESC;"
echo "   Evaluate Now: SELECT * FROM evaluate_alerts();"
echo ""
echo "⚠️  Next Steps:"
echo "   1. Configure notification endpoints (email, Slack, etc.)"
echo "   2. Set up Grafana dashboards using exported metrics"
echo "   3. Test alerting with intentional failures"
echo "   4. Configure log retention policies"
echo ""