#!/bin/bash

# Test script for enhanced --save-images flag functionality

echo "🚀 Testing Enhanced --save-images Flag"
echo "======================================"

# Kill any existing pulson processes
pkill -f "pulson serve" || true
sleep 2

cd /doc/code/pulson

# Test 1: Test raw image data support
echo ""
echo "🧪 Test 1: Raw Image Data Support"
echo "================================="

rm -f /tmp/pulson_test_raw_images.db

echo "📡 Starting server with --save-images..."
./target/debug/pulson serve --webui --db-path /tmp/pulson_test_raw_images.db --save-images &
PULSON_PID_1=$!
sleep 3

echo "👤 Setting up test user..."
./target/debug/pulson account register --username testuser --password testpass123 > /dev/null 2>&1
./target/debug/pulson account login --username testuser --password testpass123 > /dev/null 2>&1

echo "📷 Sending raw image data (3x3 RGB)..."
# Create a simple 3x3 RGB image (27 bytes: 9 pixels * 3 channels)
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 3 --height 3 --channels 3 --image-data "255,0,0,0,255,0,0,0,255,255,255,0,255,0,255,0,255,255,128,128,128,64,64,64,192,192,192"

echo "✅ Stopping server..."
kill $PULSON_PID_1
sleep 2

# Test 2: Test latest image only storage (without --save-images)
echo ""
echo "🧪 Test 2: Latest Image Only Storage (--save-images disabled)"
echo "============================================================"

rm -f /tmp/pulson_test_latest_only.db

echo "📡 Starting server WITHOUT --save-images..."
./target/debug/pulson serve --webui --db-path /tmp/pulson_test_latest_only.db &
PULSON_PID_2=$!
sleep 3

echo "👤 Setting up test user..."
./target/debug/pulson account register --username testuser --password testpass123 > /dev/null 2>&1
./target/debug/pulson account login --username testuser --password testpass123 > /dev/null 2>&1

echo "📷 Sending first image (2x2 RGB)..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 2 --height 2 --channels 3 --image-data "255,0,0,0,255,0,0,0,255,255,255,255"

echo "📷 Sending second image (2x2 RGB, should replace first)..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 2 --height 2 --channels 3 --image-data "128,128,128,64,64,64,32,32,32,16,16,16"

echo "📷 Sending third image (2x2 RGB, should replace second)..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 2 --height 2 --channels 3 --image-data "255,255,0,255,0,255,0,255,255,255,128,0"

echo "✅ Stopping server..."
kill $PULSON_PID_2
sleep 2

# Test 3: Test file path support
echo ""
echo "🧪 Test 3: File Path Support"
echo "============================"

if [ -f "test_image.png" ]; then
    rm -f /tmp/pulson_test_file_images.db

    echo "📡 Starting server with --save-images..."
    ./target/debug/pulson serve --webui --db-path /tmp/pulson_test_file_images.db --save-images &
    PULSON_PID_3=$!
    sleep 3

    echo "👤 Setting up test user..."
    ./target/debug/pulson account register --username testuser --password testpass123 > /dev/null 2>&1
    ./target/debug/pulson account login --username testuser --password testpass123 > /dev/null 2>&1

    echo "📷 Sending image from file..."
    ./target/debug/pulson pulse -d testdevice -t images --data-type image --image-file test_image.png

    echo "✅ Stopping server..."
    kill $PULSON_PID_3
    sleep 2
else
    echo "⚠️ test_image.png not found, skipping file test"
fi

# Analyze databases
echo ""
echo "🔍 Analyzing Database Contents"
echo "============================="

if command -v sqlite3 &> /dev/null; then
    echo ""
    echo "📊 Raw Image Data Database:"
    echo "============================"
    echo "Number of image records:"
    sqlite3 /tmp/pulson_test_raw_images.db "SELECT COUNT(*) FROM device_data WHERE data_type = 'image';"
    echo "Sample record:"
    sqlite3 /tmp/pulson_test_raw_images.db "SELECT device_id, topic, data_type, LENGTH(data_payload) as payload_size FROM device_data WHERE data_type = 'image' LIMIT 1;"

    echo ""
    echo "📊 Latest Image Only Database:"
    echo "=============================="
    echo "Number of image records (should be 1):"
    sqlite3 /tmp/pulson_test_latest_only.db "SELECT COUNT(*) FROM device_data WHERE data_type = 'image';"
    echo "Image record details:"
    sqlite3 /tmp/pulson_test_latest_only.db "SELECT device_id, topic, data_type, LENGTH(data_payload) as payload_size, timestamp FROM device_data WHERE data_type = 'image';"

    if [ -f "test_image.png" ]; then
        echo ""
        echo "📊 File Image Database:"
        echo "======================="
        echo "Number of image records:"
        sqlite3 /tmp/pulson_test_file_images.db "SELECT COUNT(*) FROM device_data WHERE data_type = 'image';"
        echo "Image file record details:"
        sqlite3 /tmp/pulson_test_file_images.db "SELECT device_id, topic, data_type, LENGTH(data_payload) as payload_size FROM device_data WHERE data_type = 'image' LIMIT 1;"
    fi
else
    echo "sqlite3 not available, cannot analyze database contents"
fi

# Test help output
echo ""
echo "🔍 CLI Help Output"
echo "=================="
echo "Checking new image-related flags:"
./target/debug/pulson pulse --help | grep -A 10 -B 2 "image"

echo ""
echo "🏁 Enhanced Test Complete!"
echo "=========================="
echo "Expected results:"
echo "- Raw image data should be accepted with --image-data flag"
echo "- When --save-images is disabled, only the latest image should be stored (count = 1)"
echo "- When --save-images is enabled, all images should be stored"
echo "- File paths should continue to work with --image-file flag"
echo ""
echo "Database files created:"
echo "- /tmp/pulson_test_raw_images.db"
echo "- /tmp/pulson_test_latest_only.db"
if [ -f "test_image.png" ]; then
    echo "- /tmp/pulson_test_file_images.db"
fi
