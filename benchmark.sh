#!/bin/bash

if [ $# -lt 2 ]; then
    echo "Usage: $0 <iterations> <interpreter> <source> [input]"
    exit 1
fi

iterations=$1
interpreter=$2
source=$3
input=$4

if [ ! -x "$interpreter" ]; then
    echo "Error: '$interpreter' does not exist or is not executable."
    exit 1
fi

if [ ! -f "$source" ]; then
    echo "Error: Source file '$source' does not exist."
    exit 1
fi

if [ -n "$input" ] && [ ! -f "$input" ]; then
    echo "Error: Input file '$input' does not exist."
    exit 1
fi

echo "=== Benchmark ==="
echo "Iterations:  $iterations"
echo "Interpreter: $interpreter"
echo "Source:      $source"
if [ -n "$input" ]; then
    echo "Input:       $input"
fi

echo "Warming up..."
for (( i=1; i<=$iterations; i++))
do
    echo "$i/$iterations"
    if [ -n "$input" ]; then
        cat "$input" | "./$interpreter" "$source" > /dev/null 2>&1
    else
        "./$interpreter" "$source" > /dev/null 2>&1
    fi
done

total=0

for (( i=1; i<=iterations; i++ ))
do
    begin=$(date +%s.%N)
    if [ -n "$input" ]; then
        "./$interpreter" "$source" < "$input" > /dev/null 2>&1
    else
        "./$interpreter" "$source" > /dev/null 2>&1
    fi
    end=$(date +%s.%N)
    duration=$(echo "$end - $begin" | bc)
    echo "Run $i: $duration seconds"
    total=$(echo "$total + $duration" | bc)
done

average=$(echo "scale=6; $total / $iterations" | bc)
echo "=== Result ==="
echo "Average execution time: $average seconds"
