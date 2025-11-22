#!/bin/bash
# Helper script to open log files with Log Rocket
# Usage: ./open-with-log-rocket.sh /path/to/file.log

if [ $# -eq 0 ]; then
    echo "Usage: $0 <log-file-path>"
    exit 1
fi

FILE_PATH="$1"

# Check if file exists
if [ ! -f "$FILE_PATH" ]; then
    echo "Error: File '$FILE_PATH' does not exist"
    exit 1
fi

# Open Log Rocket with the file as an argument
open -a "Log Rocket" --args "$FILE_PATH"
