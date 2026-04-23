#!/usr/bin/env bash
echo "1. All"
echo "2. Settings"
echo "3. Run init_db (refreshes test dicts)"
read -p "Select an option: " choice

export RUSTFLAGS="-A warnings"

case $choice in
    1) cargo nextest run ;;
    2) cargo nextest run settings ;;
    3) cargo nextest --lib init_db -- --ignored --nocapture ;;
    *) echo "Invalid option" ;;
esac

