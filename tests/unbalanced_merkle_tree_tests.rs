use merkle_tree::binary_merkle_tree::{UnbalancedMerkleTree, Output, process_input_to_chunks, Blake3Hasher, CHUNK_LEN, IV, ChunkState};

#[test]
fn test_unbalanced_tree_creation() {
    // Create input data that will produce these chaining values
    let mut input = Vec::new();
    for i in 1..=3 {
        let mut chunk_state = ChunkState::new(IV, (i-1) as u64, 0);
        let chunk_data = vec![i as u8; CHUNK_LEN];
        chunk_state.update(&chunk_data);
        input.extend_from_slice(&chunk_data);
    }

    // Create tree with 3 leaves (not a power of 2)
    let chunk_outputs = process_input_to_chunks(&input);
    let tree = UnbalancedMerkleTree::new_from_leaves(chunk_outputs);
    assert_eq!(tree.num_leaves(), 3);
    
    // Get BLAKE3 hash of the entire input
    let mut hasher = Blake3Hasher::new();
    hasher.update(&input);
    let mut hash = [0; 32];
    hasher.finalize(&mut hash);
    
    // Convert hash to chaining value format
    let mut blake3_chaining_value = [0u32; 8];
    for i in 0..8 {
        blake3_chaining_value[i] = u32::from_le_bytes(hash[i*4..(i+1)*4].try_into().unwrap());
    }
    
    // Compare root chaining value with BLAKE3 hash
    let root = tree.root();
    assert_eq!(root.chaining_value(), blake3_chaining_value,
        "Root chaining value {:?} does not match BLAKE3 hash {:?}",
        root.chaining_value(), blake3_chaining_value);
}

#[test]
fn test_unbalanced_tree_insert() {
    println!("\n=== Starting unbalanced tree insert test ===\n");
    
    // Create initial input data - using smaller chunks for clarity
    let mut input = Vec::new();
    for i in 1..=3 {
        println!("\nCreating chunk {}", i);
        let mut chunk_state = ChunkState::new(IV, (i-1) as u64, 0);
        let chunk_data = vec![i as u8; CHUNK_LEN];
        chunk_state.update(&chunk_data);
        println!("Chunk {} data: {:?}", i, &chunk_data);
        input.extend_from_slice(&chunk_data);
    }

    // Create tree with 3 leaves
    println!("\n--- Creating initial tree with 3 leaves ---");
    let chunk_outputs = process_input_to_chunks(&input);
    println!("Initial chunk outputs: {}", chunk_outputs.len());
    for (i, output) in chunk_outputs.iter().enumerate() {
        println!("Chunk {} cv: {:?}", i, output.chaining_value());
    }
    
    let mut tree = UnbalancedMerkleTree::new_from_leaves(chunk_outputs);
    println!("Initial tree leaves: {}", tree.num_leaves());
    println!("Initial root cv: {:?}", tree.root().chaining_value());
    
    println!("\n--- Adding fourth chunk ---");
    let mut chunk_state = ChunkState::new(IV, 3, 0);
    let chunk_data = vec![4u8; 64];
    println!("Chunk 4 data: {:?}", &chunk_data);
    chunk_state.update(&chunk_data);
    input.extend_from_slice(&chunk_data);
    
    println!("\n--- Inserting fourth leaf ---");
    tree.insert_leaf(3, chunk_state.output());
    println!("Tree leaves after insert: {}", tree.num_leaves());
    
    println!("\n--- Computing BLAKE3 hash of entire input ---");
    let mut hasher = Blake3Hasher::new();
    hasher.update(&input);
    let mut hash = [0; 32];
    hasher.finalize(&mut hash);
    
    // Convert hash to chaining value format
    let mut blake3_chaining_value = [0u32; 8];
    for i in 0..8 {
        blake3_chaining_value[i] = u32::from_le_bytes(hash[i*4..(i+1)*4].try_into().unwrap());
    }
    println!("BLAKE3 final hash cv: {:?}", blake3_chaining_value);
    
    println!("\n--- Comparing root values ---");
    let root = tree.root();
    let root_cv = root.chaining_value();
    println!("Tree root cv: {:?}", root_cv);
    println!("BLAKE3 cv:    {:?}", blake3_chaining_value);
    
    assert_eq!(root_cv, blake3_chaining_value,
        "Root chaining value does not match BLAKE3 hash");
    println!("\n=== Test completed successfully ===");
} 