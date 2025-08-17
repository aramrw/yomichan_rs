#!/bin/bash

# This script provides an interactive menu to choose which test to run.

export RUSTFLAGS="-Awarnings"
CARGO_ARGS="--release --features tracing -- --show-output"
CARGO_ARGS_IGNORED="--release --features tracing -- --show-output --ignored"

# PS3 is the prompt message for the 'select' command
PS3="Please select a test to run: "

# 'select' creates a menu from a list of options
select option in "Database Test" "Text Scanner Test" "Quit"; do
  case $option in
    "Database Test")
      echo "🚀 initializing database test..."
      cargo test dbtests::init_db $CARGO_ARGS_IGNORED
      break # Exit the loop after running
      ;;
    "Text Scanner Test")
      echo "🚀 Running text scanner test..."
      cargo test textscanner::search_dbg $CARGO_ARGS
      break # Exit the loop after running
      ;;
    "Quit")
      echo "👋 Exiting."
      break # Exit the loop
      ;;
    *)
      echo "❌ Invalid option $REPLY. Please choose 1, 2, or 3."
      ;;
  esac
done
