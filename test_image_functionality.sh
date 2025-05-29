#!/bin/bash

# Test script for image visualization functionality

echo "🚀 Testing Pulson Image Visualization"
echo "======================================"

# Kill any existing pulson processes
pkill -f "pulson serve" || true
sleep 2

# Start Pulson server in background
echo "📡 Starting Pulson server..."
cd /doc/code/pulson
./target/debug/pulson serve --webui --db-path /tmp/pulson_test.db &
PULSON_PID=$!

# Wait for server to start
echo "⏳ Waiting for server to start..."
sleep 3

# Register a test user
echo "👤 Registering test user..."
./target/debug/pulson account register --username testuser --password testpass123

# Login
echo "🔑 Logging in..."
./target/debug/pulson account login --username testuser --password testpass123

# Send a test image
echo "📷 Sending test image..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --image-file test_image.png --width 64 --height 64

# Send another image using dummy data
echo "🖼️ Sending dummy image data..."
./target/debug/pulson pulse -d testdevice -t images --data-type image --width 32 --height 32

# Send some other test data for comparison
echo "📊 Sending other test data..."
./target/debug/pulson pulse -d testdevice -t heartbeat --data-type pulse
./target/debug/pulson pulse -d testdevice -t gps --data-type gps --latitude 37.7749 --longitude=-122.4194

echo ""
echo "✅ Test data sent successfully!"
echo ""
echo "🌐 Open your browser to: http://localhost:3030"
echo "   - Login with: testuser / testpass123"
echo "   - Navigate to testdevice dashboard"
echo "   - Check the 'images' topic for image visualization"
echo ""
echo "⚠️  Server is running in background (PID: $PULSON_PID)"
echo "   To stop: kill $PULSON_PID"
echo ""
