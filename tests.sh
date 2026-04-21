#!/usr/bin/env bash
echo "--- Yomichan RS Testing Suite ---"
echo "1. Run all tests"
echo "2. Run structured search tests"
echo "3. Run database tests"
echo "4. Initialize/Refresh DB (init_db)"
read -p "Select an option: " choice

case $choice in
    1) cargo test ;;
    2) cargo test --test test_new_search_api ;;
    3) cargo test --test database_test ;;
    4) cargo test --test scanner init_db -- --ignored --nocapture ;;
    *) echo "Invalid option" ;;
esac

