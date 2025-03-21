mod binary_merkle_tree;

use rand::Rng;
use crate::binary_merkle_tree::{BinaryMerkleTree, process_input_to_chunks, ChunkState, Blake3Hasher, CHUNK_LEN, IV};

fn main() {
    println!("Testing Merkle Tree operations with random input (4096 bytes):");
    let mut rng = rand::thread_rng();
    let mut input: Vec<u8> = (0..4096).map(|_| rng.gen()).collect();
    
    // Get initial BLAKE3 hash
    let mut hasher = Blake3Hasher::new();
    hasher.update(&input);
    let mut initial_hash = [0; 32];
    hasher.finalize(&mut initial_hash);
    
    // Convert initial hash bytes to chaining value format (8 u32 values)
    let mut initial_blake3_chaining_value = [0u32; 8];
    for i in 0..8 {
        initial_blake3_chaining_value[i] = u32::from_le_bytes(initial_hash[i*4..(i+1)*4].try_into().unwrap());
    }
    println!("\nInitial BLAKE3 hash: {:?}", initial_blake3_chaining_value);
    
    // Process through Merkle tree
    let chunk_outputs = process_input_to_chunks(&input);
    let mut tree = BinaryMerkleTree::new_from_leaves(chunk_outputs);
    let initial_root = tree.root().chaining_value();
    println!("\nInitial root hash: {:?}", initial_root);
    println!("Initial match: {}", initial_root == initial_blake3_chaining_value);

    // Randomly select 5 positions to mutate
    let num_mutations = 5;
    let mut mutation_positions: Vec<usize> = (0..input.len()).collect();
    for _ in 0..num_mutations {
        let pos = rng.gen_range(0..mutation_positions.len());
        let mutation_index = mutation_positions.remove(pos);
        let original_byte = input[mutation_index];
        input[mutation_index] = original_byte ^ 0xFF; // Flip all bits
        println!("\nMutated byte at index {}: 0x{:02X} -> 0x{:02X}", 
                 mutation_index, original_byte, input[mutation_index]);

        // Find which chunk contains our mutated byte
        let chunk_index = mutation_index / CHUNK_LEN;
        println!("Mutation is in chunk {}", chunk_index);

        // Create new Output for the mutated chunk
        let mut chunk_state = ChunkState::new(IV, chunk_index as u64, 0);
        let chunk_start = chunk_index * CHUNK_LEN;
        let chunk_end = std::cmp::min(chunk_start + CHUNK_LEN, input.len());
        chunk_state.update(&input[chunk_start..chunk_end]);
        let mutated_chunk_output = chunk_state.output();

        // Update the tree with the mutated chunk
        tree.insert_leaf(chunk_index, mutated_chunk_output);
    }

    // Get mutated BLAKE3 hash
    let mut hasher = Blake3Hasher::new();
    hasher.update(&input);
    let mut mutated_hash = [0; 32];
    hasher.finalize(&mut mutated_hash);
    
    // Convert mutated hash bytes to chaining value format
    let mut mutated_blake3_chaining_value = [0u32; 8];
    for i in 0..8 {
        mutated_blake3_chaining_value[i] = u32::from_le_bytes(mutated_hash[i*4..(i+1)*4].try_into().unwrap());
    }
    println!("\nMutated BLAKE3 hash: {:?}", mutated_blake3_chaining_value);

    let mutated_root = tree.root().chaining_value();
    println!("\nRoot hash after mutations: {:?}", mutated_root);
    println!("Mutated match: {}", mutated_root == mutated_blake3_chaining_value);
}