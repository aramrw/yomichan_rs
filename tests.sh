#!/usr/bin/env bash
echo "--- Yomichan RS Testing Suite ---"
echo "1. Run all tests"
echo "2. Run structured search tests"
echo "3. Run init_db (refreshes test dicts)"
read -p "Select an option: " choice

export RUSTFLAGS="-A warnings"

case $choice in
    1) cargo test ;;
    2) cargo test --test test_structured_search ;;
    3) cargo test --lib init_db -- --ignored --nocapture ;;
    *) echo "Invalid option" ;;
esac

