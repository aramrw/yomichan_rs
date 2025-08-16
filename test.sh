RUST_BACKTRACE=1 RUSTFLAGS="-Awarnings" RUST_LOG=debug cargo test import_in_memory --release --features rayon -- --show-output 
