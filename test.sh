#!/bin/bash

# This script provides an interactive menu to choose which test to run.

export RUSTFLAGS="-Awarnings"
RUSTFLAGS="-Awarnings"
CARGO_ARGS="--release  -- --nocapture"
CARGO_ARGS_IGNORED="--release -- --nocapture --ignored --test-threads=1"

# PS3 is the prompt message for the 'select' command
PS3="Please select a test to run: "

# 'select' creates a menu from a list of options
select option in "dbtests::init_db" "textscanner::search_dbg" "quit"; do
  case $option in
    "dbtests::init_db")
      echo "🚀 initializing database test..."
      RUSTFLAGS="-Awarnings" cargo test dbtests::init_db $CARGO_ARGS_IGNORED
      break # Exit the loop after running
      ;;
    "textscanner::search_dbg")
      echo "🚀 Running text scanner test..."
      RUSTFLAGS="-Awarnings" cargo test textscanner::search_dbg $CARGO_ARGS
      break # Exit the loop after running
      ;;
    "quit")
      echo "👋 Exiting."
      break # Exit the loop
      ;;
    *)
      echo "❌ Invalid option $REPLY. Please choose 1, 2, or 3."
      ;;
  esac
done
