rm -rf ./target *.prof*

export RUSTFLAGS="-C instrument-coverage=all"

export LLVM_PROFILE_FILE="./target/test-%p-%m.profraw"

# Build the program
# cargo build

# cargo test --package fyrox-core
cargo test --package fyrox-core --package fyrox-resource

# Generate a HTML report in the coverage/ directory.
grcov . --binary-path ./target/debug/ -s . -t html --branch --ignore-not-existing -o ./target/coverage/
