#!/usr/bin/env python3
"""
Extract results from trilemma_comparison output and update HTML file
"""
import re
import sys

def extract_metrics_from_output(output_file):
    """Extract metrics from experiment output"""
    with open(output_file, 'r') as f:
        content = f.read()
    
    results = {}
    
    # Extract Performance Metrics
    perf_match = re.search(
        r'Performance Metrics.*?\n(.*?)(?=\n\n|\nTrilemma|$)',
        content,
        re.DOTALL
    )
    
    # Extract Extended Trilemma Metrics
    extended_match = re.search(
        r'Extended Trilemma Metrics.*?Scalability - 3 metrics:.*?\n(.*?)(?=\n\nSecurity|$)',
        content,
        re.DOTALL
    )
    
    # Extract Security metrics
    security_match = re.search(
        r'Security - 4 metrics:.*?\n(.*?)(?=\n\nTrilemma Scores|$)',
        content,
        re.DOTALL
    )
    
    return {
        'performance': perf_match.group(1) if perf_match else None,
        'scalability': extended_match.group(1) if extended_match else None,
        'security': security_match.group(1) if security_match else None,
    }

def parse_table_data(table_text):
    """Parse table data from text output"""
    lines = [l.strip() for l in table_text.split('\n') if l.strip() and '|' in l]
    data = []
    for line in lines[1:]:  # Skip header
        parts = [p.strip() for p in line.split('|')]
        if len(parts) >= 2:
            data.append(parts)
    return data

if __name__ == '__main__':
    output_file = '/tmp/trilemma_full_results.txt'
    html_file = 'docs/index.html'
    
    print(f"Reading results from {output_file}...")
    metrics = extract_metrics_from_output(output_file)
    
    for key, value in metrics.items():
        if value:
            print(f"\n{key.upper()}:")
            print(value[:500])
        else:
            print(f"\n{key.upper()}: Not found")
