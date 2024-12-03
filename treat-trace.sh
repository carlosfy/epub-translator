#!/bin/bash
# This script generates a CSV file from the logs, to analyze the performance of the application and DeepL's response time.

# Check if the correct number of arguments is provided
if [[ $# -ne 2 ]]; then
    echo "Usage: $0 input_file output_file"
    exit 1
fi

# Assign arguments to variables
input_log_file="$1"
output_file="$2"

# Temporary file for intermediary CSV
temp_csv_file=$(mktemp)

# Extract `[TRACE]` lines from the log file and remove the prefix
grep '^\[TRACE\]' "$input_log_file" | sed 's/^\[TRACE\]//' > "$temp_csv_file"

# Write the header to the output file
read -r header < "$temp_csv_file"

echo "$header" > "$output_file"


tail -n +2 "$temp_csv_file" | while IFS=, read -r id len error_code start_1 start_2 duration permits thread; do 
    start_combined="$start_1$start_2"
    if [[ -n $duration ]]; then
        duration=$((duration / 1000000))
    else
        duration="";
    fi
    if [[ $start_combined =~ tv_sec:\ ([0-9]+)\ tv_nsec:\ ([0-9]+) ]]; then
        tv_secs="${BASH_REMATCH[1]}"
        tv_nsecs="${BASH_REMATCH[2]}"

        tv_secs_suffix="${tv_secs: -4}"
        tv_nsecs_prefix="${tv_nsecs:0:3}"
        start="$tv_secs_suffix$tv_nsecs_prefix"
    fi
    if [[ $thread =~ ThreadId\(([0-9]+)\) ]]; then
        thread_num="${BASH_REMATCH[1]}"
    fi

    if [[ -n $start && -n $thread_num ]]; then
        line="$id,$len,$error_code,$start,$duration,$permits,$thread_num"
    else
        if [[ -n $start ]]; then 
            line="$id,$len,$error_code,$start,$duration,$permits,$thread"
        else 
            if [[ -n $thread_num ]]; then
                line="$id,$len,$error_code,0,$duration,$permits,$thread_num"
            else
                line="$id,$len,$error_code,0,$duration,$permits,$thread"
            fi
        fi
            
    fi

    echo "$line" >> "$output_file"
done

# Clean temporary file
rm "$temp_csv_file"