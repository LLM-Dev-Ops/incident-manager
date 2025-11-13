#!/bin/bash

# Test interactive Seraphina mode
echo "Testing interactive Seraphina chat..."
echo ""

# Use expect to automate the interactive session
cat > test-seraphina-interactive.exp << 'EOF'
#!/usr/bin/expect -f

set timeout 30

spawn npx flow-nexus seraphina

# Wait for the Queen's greeting
expect "*Queen Seraphina:*"
expect "*Type*exit*"

# Send first message
send "Hello Queen Seraphina!\r"
expect "*You >*"

# Wait for response
expect "*Queen Seraphina:*"
sleep 2

# Send second message to test conversation continuity
send "What are swarms?\r"
expect "*You >*"

# Wait for response
expect "*Queen Seraphina:*"
sleep 2

# Exit
send "exit\r"
expect eof
EOF

# Run the expect script
if command -v expect >/dev/null 2>&1; then
  expect test-seraphina-interactive.exp
else
  echo "Installing expect..."
  sudo apt-get update && sudo apt-get install -y expect
  expect test-seraphina-interactive.exp
fi

# Clean up
rm -f test-seraphina-interactive.exp