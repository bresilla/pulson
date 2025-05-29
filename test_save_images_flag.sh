#!/bin/bash

# Test script for --save-images flag functionality

echo "🚀 Testing Pulson --save-images Flag"
echo "===================================="

# Kill any existing pulson processes
pkill -f "pulson serve" || true
sleep 2

# Test 1: Start server with --save-images enabled
echo ""
echo "🧪 Test 1: Running server WITH --save-images flag"
echo "================================================="

cd /doc/code/pulson
rm -f /tmp/pulson_test_with_images.db

echo "📡 Starting Pulson server with --save-images..."
./target/debug/pulson serve --webui --db-path /tmp/pulson_test_with_images.db --save-images &
PULSON_PID_1=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 3

# Register and login
echo "👤 Setting up test user..."
./target/debug/pulson account register --username testuser --password testpass123 > /dev/null 2>&1
./target/debug/pulson account login --username testuser --password testpass123 > /dev/null 2>&1

# Send test image data
echo "📷 Sending test image data..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 32 --height 32

echo "✅ Stopping server with --save-images..."
kill $PULSON_PID_1
sleep 2

# Test 2: Start server without --save-images (default)
echo ""
echo "🧪 Test 2: Running server WITHOUT --save-images flag (default)"
echo "=============================================================="

rm -f /tmp/pulson_test_without_images.db

echo "📡 Starting Pulson server without --save-images..."
./target/debug/pulson serve --webui --db-path /tmp/pulson_test_without_images.db &
PULSON_PID_2=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 3

# Register and login
echo "👤 Setting up test user..."
./target/debug/pulson account register --username testuser --password testpass123 > /dev/null 2>&1
./target/debug/pulson account login --username testuser --password testpass123 > /dev/null 2>&1

# Send test image data
echo "📷 Sending test image data..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 32 --height 32

echo "✅ Stopping server without --save-images..."
kill $PULSON_PID_2
sleep 2

# Analyze the databases
echo ""
echo "🔍 Analyzing Database Contents"
echo "============================="

echo ""
echo "📊 Database WITH --save-images:"
echo "==============================="
if command -v sqlite3 &> /dev/null; then
    echo "Number of records in device_data table:"
    sqlite3 /tmp/pulson_test_with_images.db "SELECT COUNT(*) FROM device_data;"
    echo ""
    echo "Data types stored:"
    sqlite3 /tmp/pulson_test_with_images.db "SELECT data_type, COUNT(*) FROM device_data GROUP BY data_type;"
    echo ""
    echo "Sample data (first record):"
    sqlite3 /tmp/pulson_test_with_images.db "SELECT id, device_id, topic, data_type, CASE WHEN LENGTH(data) > 100 THEN SUBSTR(data, 1, 100) || '...' ELSE data END as truncated_data FROM device_data LIMIT 1;"
else
    echo "sqlite3 not available, cannot analyze database contents"
fi

echo ""
echo "📊 Database WITHOUT --save-images:"
echo "=================================="
if command -v sqlite3 &> /dev/null; then
    echo "Number of records in device_data table:"
    sqlite3 /tmp/pulson_test_without_images.db "SELECT COUNT(*) FROM device_data;"
    echo ""
    echo "Data types stored:"
    sqlite3 /tmp/pulson_test_without_images.db "SELECT data_type, COUNT(*) FROM device_data GROUP BY data_type;"
    echo ""
    echo "Sample data (first record):"
    sqlite3 /tmp/pulson_test_without_images.db "SELECT id, device_id, topic, data_type, data FROM device_data LIMIT 1;"
else
    echo "sqlite3 not available, cannot analyze database contents"
fi

echo ""
echo "🏁 Test Complete!"
echo "================="
echo "Expected results:"
echo "- Database WITH --save-images should contain 'image' data type with full pixel data"
echo "- Database WITHOUT --save-images should contain 'event' data type with metadata only"
echo ""
echo "Database files created:"
echo "- /tmp/pulson_test_with_images.db"
echo "- /tmp/pulson_test_without_images.db"
