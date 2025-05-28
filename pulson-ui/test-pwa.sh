#!/bin/bash

# PWA Test Script for Pulson
# This script helps you test your Progressive Web App implementation

echo "🚀 Pulson PWA Test Suite"
echo "========================"

# Check if we're in the correct directory
if [ ! -f "static/manifest.json" ]; then
    echo "❌ Error: Please run this script from the pulson-ui directory"
    exit 1
fi

echo "📋 Checking PWA files..."

# Check manifest.json
if [ -f "static/manifest.json" ]; then
    echo "✅ manifest.json found"
else
    echo "❌ manifest.json missing"
fi

# Check service worker
if [ -f "static/sw.js" ]; then
    echo "✅ sw.js found"
else
    echo "❌ sw.js missing"
fi

# Check offline page
if [ -f "static/offline.html" ]; then
    echo "✅ offline.html found"
else
    echo "❌ offline.html missing"
fi

# Check PWA manager
if [ -f "static/pwa-manager.js" ]; then
    echo "✅ pwa-manager.js found"
else
    echo "❌ pwa-manager.js missing"
fi

# Check PWA styles
if [ -f "static/styles/pwa.css" ]; then
    echo "✅ pwa.css found"
else
    echo "❌ pwa.css missing"
fi

echo ""
echo "🔍 Validating PWA manifest..."

# Validate manifest.json
if command -v jq &> /dev/null; then
    if jq empty static/manifest.json 2>/dev/null; then
        echo "✅ manifest.json is valid JSON"
        
        # Check required PWA fields
        name=$(jq -r '.name' static/manifest.json)
        start_url=$(jq -r '.start_url' static/manifest.json)
        display=$(jq -r '.display' static/manifest.json)
        icons=$(jq -r '.icons | length' static/manifest.json)
        
        echo "   Name: $name"
        echo "   Start URL: $start_url"
        echo "   Display: $display"
        echo "   Icons: $icons"
        
        if [ "$icons" -gt 0 ]; then
            echo "✅ Icons configured"
        else
            echo "❌ No icons configured"
        fi
    else
        echo "❌ manifest.json is invalid JSON"
    fi
else
    echo "⚠️  jq not found, skipping manifest validation"
fi

echo ""
echo "🌐 Testing local server setup..."

# Check if a local server is running
if curl -s http://localhost:8000 &> /dev/null; then
    echo "✅ Local server running on port 8000"
elif curl -s http://localhost:3000 &> /dev/null; then
    echo "✅ Local server running on port 3000"
elif curl -s http://localhost:8080 &> /dev/null; then
    echo "✅ Local server running on port 8080"
else
    echo "❌ No local server detected"
    echo "   💡 To test your PWA, you need to serve it over HTTP/HTTPS"
    echo "   💡 Try: python3 -m http.server 8000 (from static/ directory)"
    echo "   💡 Or: npx serve static/"
fi

echo ""
echo "📱 PWA Testing Instructions:"
echo "============================"
echo "1. Serve your app over HTTP/HTTPS (required for PWA features)"
echo "2. Open your browser and navigate to your app"
echo "3. Visit /static/pwa-test.html for detailed PWA testing"
echo "4. Open browser DevTools > Application tab to inspect:"
echo "   - Service Worker registration"
echo "   - Cache storage"
echo "   - Manifest"
echo "5. Test on mobile devices for the best PWA experience"
echo ""
echo "🔧 Lighthouse PWA Audit:"
echo "Run a Lighthouse audit in Chrome DevTools for a comprehensive PWA score"
echo ""
echo "📊 Quick Server Commands:"
echo "# From pulson-ui/static/ directory:"
echo "python3 -m http.server 8000"
echo "# or"
echo "npx serve ."
echo ""
echo "Then visit: http://localhost:8000/pwa-test.html"
