# BLAKE3 Merkle Tree Implementation

This project implements a Merkle Tree using the BLAKE3 cryptographic hash function. It includes both balanced and unbalanced Merkle tree implementations with support for efficient updates and insertions.

## Features

- Balanced Binary Merkle Tree implementation
- Unbalanced Merkle Tree implementation (for non-power-of-two number of leaves)
- BLAKE3 hashing algorithm integration
- Support for single leaf insertion and bulk insertions
- Efficient parent node computation and tree updates
- Comprehensive test suite

## Usage

```rust
use merkle_tree::binary_merkle_tree::{UnbalancedMerkleTree, Output, process_input_to_chunks};

// Create input data
let input = vec![1, 2, 3]; // Your input data

// Process input into chunks and create tree
let chunk_outputs = process_input_to_chunks(&input);
let mut tree = UnbalancedMerkleTree::new_from_leaves(chunk_outputs);

// Get root hash
let root = tree.root();
let root_hash = root.chaining_value();

// Insert new leaf
let new_leaf = /* create new leaf output */;
tree.insert_leaf(3, new_leaf);
```

## Building and Testing

```bash
# Build the project
cargo build

# Run tests
cargo test

# Run specific test
cargo test test_unbalanced_tree_insert
```

## License

MIT License 