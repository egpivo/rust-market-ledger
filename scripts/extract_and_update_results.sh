#!/bin/bash
# Script to extract results from trilemma_comparison output and update HTML

OUTPUT_FILE="/tmp/trilemma_full_results.txt"
HTML_FILE="docs/index.html"

echo "Waiting for experiment to complete..."
while ps aux | grep -q "[t]rilemma_comparison"; do
    sleep 30
    echo "Still running... $(date)"
done

echo "Experiment completed! Extracting results..."

# Extract Performance Metrics section
echo "Extracting Performance Metrics..."
tail -1000 "$OUTPUT_FILE" | grep -A50 "Performance Metrics" | head -30

# Extract Extended Trilemma Metrics
echo "Extracting Extended Trilemma Metrics..."
tail -1000 "$OUTPUT_FILE" | grep -A200 "Extended Trilemma Metrics"

echo "Results extracted. Please review and update HTML manually."
