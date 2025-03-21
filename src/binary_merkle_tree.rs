use std::collections::VecDeque;
use std::iter::FromIterator;
use core::cmp::min;

pub const OUT_LEN: usize = 32;
pub const KEY_LEN: usize = 32;
pub const BLOCK_LEN: usize = 64;
pub const CHUNK_LEN: usize = 1024;

const CHUNK_START: u32 = 1 << 0;
const CHUNK_END: u32 = 1 << 1;
const PARENT: u32 = 1 << 2;
pub const ROOT: u32 = 1 << 3;
const KEYED_HASH: u32 = 1 << 4;
const DERIVE_KEY_CONTEXT: u32 = 1 << 5;
const DERIVE_KEY_MATERIAL: u32 = 1 << 6;

pub const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

// Each chunk or parent node can produce either an 8-word chaining value or, by
// setting the ROOT flag, any number of final output bytes. The Output struct
// captures the state just prior to choosing between those two possibilities.
#[derive(Debug, Clone, Copy)]
pub struct Output {
    pub input_chaining_value: [u32; 8],
    pub block_words: [u32; 16],
    pub counter: u64,
    pub block_len: u32,
    pub flags: u32,
}

impl Output {
    pub fn chaining_value(&self) -> [u32; 8] {
        let cv = first_8_words(compress(
            &self.input_chaining_value,
            &self.block_words,
            self.counter,
            self.block_len,
            self.flags,
        ));
        println!("Output chaining_value: input_cv={:?}, block_words={:?}, counter={}, block_len={}, flags={:b} => cv={:?}",
            self.input_chaining_value, self.block_words, self.counter, self.block_len, self.flags, cv);
        cv
    }

    pub fn root_output_bytes(&self, out_slice: &mut [u8]) {
        let mut output_block_counter = 0;
        for out_block in out_slice.chunks_mut(2 * OUT_LEN) {
            let words = compress(
                &self.input_chaining_value,
                &self.block_words,
                output_block_counter,
                self.block_len,
                self.flags | ROOT,
            );
            // The output length might not be a multiple of 4.
            for (word, out_word) in words.iter().zip(out_block.chunks_mut(4)) {
                out_word.copy_from_slice(&word.to_le_bytes()[..out_word.len()]);
            }
            output_block_counter += 1;
        }
    }
}

pub fn parent_output(
    left_child_cv: [u32; 8],
    right_child_cv: [u32; 8],
    key_words: [u32; 8],
    flags: u32,
) -> Output {
    println!("Creating parent node: left_cv={:?}, right_cv={:?}, key={:?}, flags={:b}",
        left_child_cv, right_child_cv, key_words, flags);
    let mut block_words = [0; 16];
    block_words[..8].copy_from_slice(&left_child_cv);
    block_words[8..].copy_from_slice(&right_child_cv);
    Output {
        input_chaining_value: key_words,
        block_words,
        counter: 0,                  // Always 0 for parent nodes.
        block_len: BLOCK_LEN as u32, // Always BLOCK_LEN (64) for parent nodes.
        flags: PARENT | flags,
    }
}

fn first_8_words(compression_output: [u32; 16]) -> [u32; 8] {
    compression_output[0..8].try_into().unwrap()
}

fn compress(
    chaining_value: &[u32; 8],
    block_words: &[u32; 16],
    counter: u64,
    block_len: u32,
    flags: u32,
) -> [u32; 16] {
    let counter_low = counter as u32;
    let counter_high = (counter >> 32) as u32;
    #[rustfmt::skip]
    let mut state = [
        chaining_value[0], chaining_value[1], chaining_value[2], chaining_value[3],
        chaining_value[4], chaining_value[5], chaining_value[6], chaining_value[7],
        IV[0],             IV[1],             IV[2],             IV[3],
        counter_low,       counter_high,      block_len,         flags,
    ];
    let mut block = *block_words;

    round(&mut state, &block); // round 1
    permute(&mut block);
    round(&mut state, &block); // round 2
    permute(&mut block);
    round(&mut state, &block); // round 3
    permute(&mut block);
    round(&mut state, &block); // round 4
    permute(&mut block);
    round(&mut state, &block); // round 5
    permute(&mut block);
    round(&mut state, &block); // round 6
    permute(&mut block);
    round(&mut state, &block); // round 7

    for i in 0..8 {
        state[i] ^= state[i + 8];
        state[i + 8] ^= chaining_value[i];
    }
    state
}

const MSG_PERMUTATION: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

fn permute(m: &mut [u32; 16]) {
    let mut permuted = [0; 16];
    for i in 0..16 {
        permuted[i] = m[MSG_PERMUTATION[i]];
    }
    *m = permuted;
}

fn round(state: &mut [u32; 16], m: &[u32; 16]) {
    // Mix the columns.
    g(state, 0, 4, 8, 12, m[0], m[1]);
    g(state, 1, 5, 9, 13, m[2], m[3]);
    g(state, 2, 6, 10, 14, m[4], m[5]);
    g(state, 3, 7, 11, 15, m[6], m[7]);
    // Mix the diagonals.
    g(state, 0, 5, 10, 15, m[8], m[9]);
    g(state, 1, 6, 11, 12, m[10], m[11]);
    g(state, 2, 7, 8, 13, m[12], m[13]);
    g(state, 3, 4, 9, 14, m[14], m[15]);
}

fn g(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(mx);
    state[d] = (state[d] ^ state[a]).rotate_right(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(12);
    state[a] = state[a].wrapping_add(state[b]).wrapping_add(my);
    state[d] = (state[d] ^ state[a]).rotate_right(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] = (state[b] ^ state[c]).rotate_right(7);
}

fn words_from_little_endian_bytes(bytes: &[u8], words: &mut [u32]) {
    debug_assert_eq!(bytes.len(), 4 * words.len());
    for (four_bytes, word) in bytes.chunks_exact(4).zip(words) {
        *word = u32::from_le_bytes(four_bytes.try_into().unwrap());
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkState {
    pub chaining_value: [u32; 8],
    pub chunk_counter: u64,
    pub block: [u8; BLOCK_LEN],
    pub block_len: u8,
    pub blocks_compressed: u8,
    pub flags: u32,
}

impl ChunkState {
    pub fn new(key_words: [u32; 8], chunk_counter: u64, flags: u32) -> Self {
        Self {
            chaining_value: key_words,
            chunk_counter,
            block: [0; BLOCK_LEN],
            block_len: 0,
            blocks_compressed: 0,
            flags,
        }
    }

    pub fn len(&self) -> usize {
        BLOCK_LEN * self.blocks_compressed as usize + self.block_len as usize
    }

    pub fn start_flag(&self) -> u32 {
        if self.blocks_compressed == 0 {
            CHUNK_START
        } else {
            0
        }
    }

    pub fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            // If the block buffer is full, compress it and clear it. More
            // input is coming, so this compression is not CHUNK_END.
            if self.block_len as usize == BLOCK_LEN {
                let mut block_words = [0; 16];
                words_from_little_endian_bytes(&self.block, &mut block_words);
                self.chaining_value = first_8_words(compress(
                    &self.chaining_value,
                    &block_words,
                    self.chunk_counter,
                    BLOCK_LEN as u32,
                    self.flags | self.start_flag() as u32,
                ));
                self.blocks_compressed += 1;
                self.block = [0; BLOCK_LEN];
                self.block_len = 0;
            }

            // Copy input bytes into the block buffer.
            let want = BLOCK_LEN - self.block_len as usize;
            let take = min(want, input.len());
            self.block[self.block_len as usize..][..take].copy_from_slice(&input[..take]);
            self.block_len += take as u8;
            input = &input[take..];
        }
    }

    pub fn output(&self) -> Output {
        let mut block_words = [0; 16];
        words_from_little_endian_bytes(&self.block, &mut block_words);
        println!("ChunkState output: cv={:?}, counter={}, block={:?}, block_len={}, blocks_compressed={}, flags={:b}",
            self.chaining_value, self.chunk_counter, self.block, self.block_len, self.blocks_compressed, self.flags);
        let output = Output {
            input_chaining_value: self.chaining_value,
            block_words,
            counter: self.chunk_counter,
            block_len: self.block_len as u32,
            flags: self.flags | self.start_flag() as u32 | CHUNK_END as u32,
        };
        output
    }
}

pub fn parent_cv(
    left_child_cv: [u32; 8],
    right_child_cv: [u32; 8],
    key_words: [u32; 8],
    flags: u32,
) -> [u32; 8] {
    parent_output(left_child_cv, right_child_cv, key_words, flags).chaining_value()
}

/// An incremental hasher that can accept any number of writes.
pub struct Blake3Hasher {
    chunk_state: ChunkState,
    key_words: [u32; 8],
    cv_stack: [[u32; 8]; 54], // Space for 54 subtree chaining values:
    cv_stack_len: u8,         // 2^54 * CHUNK_LEN = 2^64
    flags: u32,
}

impl Blake3Hasher {
    fn new_internal(key_words: [u32; 8], flags: u32) -> Self {
        Self {
            chunk_state: ChunkState::new(key_words, 0, flags),
            key_words,
            cv_stack: [[0; 8]; 54],
            cv_stack_len: 0,
            flags,
        }
    }

    /// Construct a new `Hasher` for the regular hash function.
    pub fn new() -> Self {
        Self::new_internal(IV, 0)
    }

    /// Construct a new `Hasher` for the keyed hash function.
    pub fn new_keyed(key: &[u8; KEY_LEN]) -> Self {
        let mut key_words = [0; 8];
        words_from_little_endian_bytes(key, &mut key_words);
        Self::new_internal(key_words, KEYED_HASH)
    }

    /// Construct a new `Hasher` for the key derivation function. The context
    /// string should be hardcoded, globally unique, and application-specific.
    pub fn new_derive_key(context: &str) -> Self {
        let mut context_hasher = Self::new_internal(IV, DERIVE_KEY_CONTEXT);
        context_hasher.update(context.as_bytes());
        let mut context_key = [0; KEY_LEN];
        context_hasher.finalize(&mut context_key);
        let mut context_key_words = [0; 8];
        words_from_little_endian_bytes(&context_key, &mut context_key_words);
        Self::new_internal(context_key_words, DERIVE_KEY_MATERIAL)
    }

    fn push_stack(&mut self, cv: [u32; 8]) {
        self.cv_stack[self.cv_stack_len as usize] = cv;
        self.cv_stack_len += 1;
    }

    fn pop_stack(&mut self) -> [u32; 8] {
        self.cv_stack_len -= 1;
        self.cv_stack[self.cv_stack_len as usize]
    }

    // Section 5.1.2 of the BLAKE3 spec explains this algorithm in more detail.
    fn add_chunk_chaining_value(&mut self, mut new_cv: [u32; 8], mut total_chunks: u64) {
        // This chunk might complete some subtrees. For each completed subtree,
        // its left child will be the current top entry in the CV stack, and
        // its right child will be the current value of `new_cv`. Pop each left
        // child off the stack, merge it with `new_cv`, and overwrite `new_cv`
        // with the result. After all these merges, push the final value of
        // `new_cv` onto the stack. The number of completed subtrees is given
        // by the number of trailing 0-bits in the new total number of chunks.
        while total_chunks & 1 == 0 {
            new_cv = parent_cv(self.pop_stack(), new_cv, self.key_words, self.flags);
            total_chunks >>= 1;
        }
        self.push_stack(new_cv);
    }

    /// Add input to the hash state. This can be called any number of times.
    pub fn update(&mut self, mut input: &[u8]) {
        while !input.is_empty() {
            // If the current chunk is complete, finalize it and reset the
            // chunk state. More input is coming, so this chunk is not ROOT.
            if self.chunk_state.len() == CHUNK_LEN {
                let chunk_cv = self.chunk_state.output().chaining_value();
                let total_chunks = self.chunk_state.chunk_counter + 1;
                self.add_chunk_chaining_value(chunk_cv, total_chunks);
                self.chunk_state = ChunkState::new(self.key_words, total_chunks, self.flags);
            }

            // Compress input bytes into the current chunk state.
            let want = CHUNK_LEN - self.chunk_state.len();
            let take = min(want, input.len());
            self.chunk_state.update(&input[..take]);
            input = &input[take..];
        }
    }

    /// Finalize the hash and write any number of output bytes.
    pub fn finalize(&self, out_slice: &mut [u8]) {
        // Starting with the Output from the current chunk, compute all the
        // parent chaining values along the right edge of the tree, until we
        // have the root Output.
        println!("\nBLAKE3 Finalization:");
        let mut output = self.chunk_state.output();
        println!("Initial chunk output cv: {:?}", output.chaining_value());
        let mut parent_nodes_remaining = self.cv_stack_len as usize;
        while parent_nodes_remaining > 0 {
            parent_nodes_remaining -= 1;
            let stack_cv = self.cv_stack[parent_nodes_remaining];
            println!("Combining with stack cv[{}]: {:?}", parent_nodes_remaining, stack_cv);
            output = parent_output(
                self.cv_stack[parent_nodes_remaining],
                output.chaining_value(),
                self.key_words,
                self.flags,
            );
            println!("New output cv: {:?}", output.chaining_value());
        }
        output.root_output_bytes(out_slice);
    }
}

#[derive(Debug, Clone)]
pub struct BinaryMerkleTree {
    pub tree: Vec<Output>,
}

impl BinaryMerkleTree {
    pub fn new_from_leaves(leaves: Vec<Output>) -> BinaryMerkleTree {
        // Initialize a zero vector with the correct number of nodes
        let number_of_leaves = leaves.len().next_power_of_two();
        let mut tree = Self::new_empty(number_of_leaves as u64);

        tree.create_tree_from_leaves(leaves);
        tree
    }

    pub fn root(&self) -> Output {
        let mut root = self.tree[1];
        // Apply ROOT flag to the final root output
        root.flags |= ROOT;
        root
    }

    pub fn num_leaves(&self) -> usize {
        self.tree.len() / 2
    }

    pub fn get_tree_length(&self) -> usize {
        self.tree.len() - 1 // Minus one because the tree is 1-indexed
    }

    pub fn new_empty(number_of_leaves: u64) -> Self {
        assert!(number_of_leaves.is_power_of_two());
        let empty_output = Output {
            input_chaining_value: IV,
            block_words: [0; 16],
            counter: 0,
            block_len: 64,
            flags: 0,
        };
        let tree: Vec<Output> = vec![empty_output; 2 * number_of_leaves as usize];
        BinaryMerkleTree { tree }
    }

    // The parent of a node is always at node_index / 2
    fn get_parent_index(index: usize) -> usize {
        index >> 1
    }

    fn create_tree_from_leaves(&mut self, leaves: Vec<Output>) {
        // Copy the leaves into the end of the tree
        let number_of_leaves = leaves.len();
        self.tree
            .splice(self.tree.capacity() - number_of_leaves.., leaves);
        // If there is only one leaf (plus the filler first node), the tree is simply that leaf
        if number_of_leaves == 1 {
            return;
        }

        // Build ancestors
        let leaf_start_index = self.get_tree_length() / 2 + 1;
        let leaves_with_indices = self.tree[leaf_start_index..]
            .iter()
            .copied()
            .zip(leaf_start_index..leaf_start_index + number_of_leaves);
        let mut hash_queue = VecDeque::from_iter(leaves_with_indices);
        while hash_queue.len() > 1 {
            let (left_child, left_index) = hash_queue.pop_front().unwrap();
            let (right_child, _right_index) = hash_queue.pop_front().unwrap();
            let parent_output = parent_output(left_child.chaining_value(), right_child.chaining_value(), IV, 0);
            let parent_index = BinaryMerkleTree::get_parent_index(left_index);
            self.tree[parent_index] = parent_output;
            hash_queue.push_back((parent_output, parent_index));
        }
    }

    pub fn insert_leaf(&mut self, leaf_index: usize, leaf_output: Output) {
        let real_leaf_index = leaf_index + self.num_leaves();
        self.tree[real_leaf_index] = leaf_output;

        let mut current_index = real_leaf_index;
        while current_index > 1 {
            // Update parent
            let parent_index = BinaryMerkleTree::get_parent_index(current_index);
            let (left_node_index, right_node_index) =
                self.get_left_and_right_node_indices_from_index(current_index);
            let left_node = &self.tree[left_node_index];
            let right_node = &self.tree[right_node_index];

            let parent_output = parent_output(left_node.chaining_value(), right_node.chaining_value(), IV, 0);
            self.tree[parent_index] = parent_output;
            current_index = parent_index;
        }
    }

    /// Bulk insert leaves and propogate hash updates to all ancestors.
    /// This method avoid updating shared parents if given two direct siblings to update.
    /// Leaf_index input should be 0-indexed where the first leaf would be entered as index 0
    pub fn bulk_insert_leaves<I, J>(
        &mut self,
        leaf_indices_iter: I,
        leaf_hashes_iter: J,
    ) -> Option<()>
    where
        I: Iterator<Item = usize>,
        J: Iterator<Item = Output>,
    {
        // Check if sorted
        let leaf_offset = self.num_leaves();
        let leaf_indices = leaf_indices_iter
            .map(|input_index| input_index + leaf_offset)
            .collect::<Vec<_>>();

        // In-line our own sort checker because Rust's is_sorted is not yet stable.
        fn is_sorted(leaf_indices: &[usize]) -> bool {
            (0..leaf_indices.len() - 1).all(|i| leaf_indices[i] < leaf_indices[i + 1])
        }
        if !is_sorted(&leaf_indices) {
            return None;
        }

        // Insert all leaf nodes
        for (leaf_index, updated_leaf_hash) in leaf_indices.iter().zip(leaf_hashes_iter) {
            self.tree[*leaf_index] = updated_leaf_hash;
        }

        // Update ancestors based on sorted leaf indices
        let mut update_queue = VecDeque::from(leaf_indices);
        while let Some(current_index) = update_queue.pop_front() {
            // Break if the root is reached
            if current_index == 1 {
                break;
            }

            // If the next ancestor to update is the sibling's, pop it from the queue
            // since it will have the same parent as the current node
            let sibling_index = BinaryMerkleTree::get_sibling_index(current_index);
            if let Some(&next_index) = update_queue.front() {
                if next_index == sibling_index {
                    update_queue.pop_front();
                }
            }

            let (left_node_index, right_node_index) =
                self.get_left_and_right_node_indices_from_index(current_index);
            let left_node = self.tree[left_node_index];
            let right_node = self.tree[right_node_index];

            let parent_output = parent_output(left_node.chaining_value(), right_node.chaining_value(), IV, 0);
            let parent_index = BinaryMerkleTree::get_parent_index(current_index);
            self.tree[parent_index] = parent_output;
            update_queue.push_back(parent_index);
        }

        Some(())
    }

    fn get_sibling_index(index: usize) -> usize {
        // Bit-wise XOR to get the sibling index
        // Example: Sibling of index 4(0b100) is 5(0b101) and sibling of index 5(0b101) is 4(0b100)
        index ^ 1
    }

    fn is_left(index: usize) -> bool {
        // All left-children have an even node index
        index % 2 == 0
    }

    /// Given an index of the current node, identify its direct sibling,
    /// identify which node is left, which is right, and return them.
    fn get_left_and_right_node_indices_from_index(&self, current_index: usize) -> (usize, usize) {
        let sibling_index = BinaryMerkleTree::get_sibling_index(current_index);

        // Use boolean indexing to avoid if statement branching
        let node_pair = [current_index, sibling_index]; // Stack allocation

        // If the sibling is the left child, is_left returns 1 and gets the sibling
        // If the sibling is the right child, is_left returns 0 and gets the node to update (the left child)
        let left_node_index = node_pair[BinaryMerkleTree::is_left(sibling_index) as usize];

        // If the node to update is the left child, is_left returns 1 and gets the sibling (the right child)
        // If the node to update is the right child, is_left returns 0 and gets the node to update
        let right_node_index = node_pair[BinaryMerkleTree::is_left(current_index) as usize];

        (left_node_index, right_node_index)
    }
}

/// Process arbitrary input bytes into a vector of Output structs.
/// This function:
/// 1. Splits input into chunks of 1024 bytes
/// 2. For each chunk, splits into blocks of 64 bytes
/// 3. Creates a ChunkState for each chunk and processes its blocks
/// 4. Returns a vector of Output structs ready for Merkle tree construction
pub fn process_input_to_chunks(input: &[u8]) -> Vec<Output> {
    const CHUNK_LEN: usize = 1024;
    const BLOCK_LEN: usize = 64;
    let mut outputs = Vec::new();
    let mut chunk_state = ChunkState::new(IV, 0, 0);
    let mut input = input;

    while !input.is_empty() {
        // If the current chunk is complete, finalize it and reset the
        // chunk state. More input is coming, so this chunk is not ROOT.
        if chunk_state.len() == CHUNK_LEN {
            let chunk_output = chunk_state.output();
            outputs.push(chunk_output);
            let total_chunks = chunk_state.chunk_counter + 1;
            chunk_state = ChunkState::new(IV, total_chunks, 0);
        }

        // Compress input bytes into the current chunk state.
        let want = CHUNK_LEN - chunk_state.len();
        let take = min(want, input.len());
        chunk_state.update(&input[..take]);
        input = &input[take..];
    }

    // Add the final chunk if it's not empty
    if chunk_state.len() > 0 {
        let chunk_output = chunk_state.output();
        outputs.push(chunk_output);
    }
    
    outputs
}

#[derive(Debug, Clone)]
pub struct UnbalancedMerkleTree {
    tree: Vec<Output>,
    actual_leaves: usize,
}

impl UnbalancedMerkleTree {
    pub fn new_from_leaves(leaves: Vec<Output>) -> Self {
        let actual_leaves = leaves.len();
        // Calculate the next power of two to allocate enough space
        let number_of_leaves = leaves.len().next_power_of_two();
        let mut tree = vec![Output {
            input_chaining_value: IV,
            block_words: [0; 16],
            counter: 0,
            block_len: 64,
            flags: 0,
        }; 2 * number_of_leaves];

        // Create a new tree with the actual number of leaves
        let mut binary_tree = UnbalancedMerkleTree { 
            tree,
            actual_leaves,
        };
        binary_tree.create_tree_from_leaves(leaves);
        binary_tree
    }

    pub fn root(&self) -> Output {
        let mut root = self.tree[1];
        // Apply ROOT flag to the final root output
        root.flags |= ROOT;
        root
    }

    pub fn num_leaves(&self) -> usize {
        self.actual_leaves
    }

    fn create_tree_from_leaves(&mut self, leaves: Vec<Output>) {
        // Copy the actual leaves into the end of the tree
        let leaf_start_index = self.tree.len() / 2;
        for (i, leaf) in leaves.into_iter().enumerate() {
            self.tree[leaf_start_index + i] = leaf;
        }

        // If there is only one leaf, the tree is simply that leaf
        if self.actual_leaves == 1 {
            self.tree[1] = self.tree[leaf_start_index];
            return;
        }

        // Build ancestors level by level, from bottom to top
        let mut current_level_start = leaf_start_index;
        let mut nodes_at_current_level = self.actual_leaves;
        
        while current_level_start > 1 {
            let parent_level_start = current_level_start / 2;
            let nodes_in_parent_level = (nodes_at_current_level + 1) / 2;

            for i in 0..nodes_in_parent_level {
                let left_index = current_level_start + 2 * i;
                let right_index = left_index + 1;
                let parent_index = parent_level_start + i;

                // For the last node in a level, if it doesn't have a right sibling,
                // promote the left node directly to be the parent
                if 2 * i + 1 >= nodes_at_current_level {
                    self.tree[parent_index] = self.tree[left_index];
                } else {
                    // If we have both left and right children, create a parent node
                    self.tree[parent_index] = parent_output(
                        self.tree[left_index].chaining_value(),
                        self.tree[right_index].chaining_value(),
                        IV,
                        0,
                    );
                }
            }
            current_level_start = parent_level_start;
            nodes_at_current_level = nodes_in_parent_level;
        }
    }

    pub fn insert_leaf(&mut self, leaf_index: usize, leaf_output: Output) {
        println!("\nInserting leaf {} into unbalanced tree:", leaf_index);
        println!("Leaf output cv: {:?}", leaf_output.chaining_value());
        
        if leaf_index >= self.actual_leaves {
            // Extend the tree if inserting beyond current leaves
            let new_actual_leaves = leaf_index + 1;
            let new_size = new_actual_leaves.next_power_of_two() * 2;
            println!("Resizing tree: actual_leaves {} -> {}, size {} -> {}", 
                self.actual_leaves, new_actual_leaves, self.tree.len(), new_size);
            if new_size > self.tree.len() {
                self.tree.resize(new_size, self.tree[0]);
            }
            self.actual_leaves = new_actual_leaves;
        }

        let leaf_start = self.tree.len() / 2;
        let real_leaf_index = leaf_index + leaf_start;
        println!("Real leaf index: {} (leaf_start={})", real_leaf_index, leaf_start);
        self.tree[real_leaf_index] = leaf_output;

        let mut current_index = real_leaf_index;
        while current_index > 1 {
            let parent_index = current_index / 2;
            let left_index = parent_index * 2;
            let right_index = left_index + 1;

            println!("\nProcessing node {}: parent={}, left={}, right={}", 
                current_index, parent_index, left_index, right_index);

            // Check if there is a valid right sibling
            let right_leaf_index = right_index - leaf_start;
            let has_right_sibling = right_leaf_index < self.actual_leaves;
            println!("Right sibling check: right_leaf_index={}, has_right_sibling={}", 
                right_leaf_index, has_right_sibling);

            if has_right_sibling {
                // Create a parent node combining both children
                println!("Creating parent node with both children:");
                println!("  Left  node cv: {:?}", self.tree[left_index].chaining_value());
                println!("  Right node cv: {:?}", self.tree[right_index].chaining_value());
                self.tree[parent_index] = parent_output(
                    self.tree[left_index].chaining_value(),
                    self.tree[right_index].chaining_value(),
                    IV,
                    0,
                );
                println!("  Parent node cv: {:?}", self.tree[parent_index].chaining_value());
            } else {
                // No right sibling, promote the left node directly
                println!("No right sibling, promoting left node:");
                println!("  Left node cv: {:?}", self.tree[left_index].chaining_value());
                self.tree[parent_index] = self.tree[left_index];
                println!("  Parent node cv: {:?}", self.tree[parent_index].chaining_value());
            }
            current_index = parent_index;
        }
        println!("Final root cv: {:?}", self.tree[1].chaining_value());
    }

    pub fn bulk_insert_leaves<I, J>(
        &mut self,
        leaf_indices_iter: I,
        leaf_hashes_iter: J,
    ) -> Option<()>
    where
        I: Iterator<Item = usize>,
        J: Iterator<Item = Output>,
    {
        // Helper function to check if indices are sorted
        fn is_sorted(indices: &[usize]) -> bool {
            indices.windows(2).all(|w| w[0] < w[1])
        }

        // Collect indices and check if sorted
        let leaf_indices: Vec<_> = leaf_indices_iter.collect();
        if !is_sorted(&leaf_indices) {
            return None;
        }

        // Find maximum leaf index and resize if needed
        if let Some(&max_index) = leaf_indices.iter().max() {
            if max_index >= self.actual_leaves {
                let new_actual_leaves = max_index + 1;
                let new_size = new_actual_leaves.next_power_of_two() * 2;
                if new_size > self.tree.len() {
                    self.tree.resize(new_size, self.tree[0]);
                }
                self.actual_leaves = new_actual_leaves;
            }
        }

        // Insert all leaf nodes
        let leaf_start = self.tree.len() / 2;
        for (leaf_index, updated_leaf_hash) in leaf_indices.iter().zip(leaf_hashes_iter) {
            self.tree[leaf_start + leaf_index] = updated_leaf_hash;
        }

        // Update ancestors using a queue to avoid duplicate updates
        let mut update_queue = VecDeque::from(leaf_indices);
        while let Some(leaf_index) = update_queue.pop_front() {
            let current_index = leaf_start + leaf_index;
            if current_index <= 1 {
                break;
            }

            let parent_index = current_index / 2;
            let left_index = parent_index * 2;
            let right_index = left_index + 1;

            // Skip if the next node is this node's sibling (they share a parent)
            if let Some(&next_leaf_index) = update_queue.front() {
                if leaf_start + next_leaf_index == right_index {
                    update_queue.pop_front();
                }
            }

            // Check if there is a valid right sibling
            let right_leaf_index = right_index - leaf_start;
            let has_right_sibling = right_leaf_index < self.actual_leaves;

            if has_right_sibling {
                // Create a parent node combining both children
                self.tree[parent_index] = parent_output(
                    self.tree[left_index].chaining_value(),
                    self.tree[right_index].chaining_value(),
                    IV,
                    0,
                );
            } else {
                // No right sibling, promote the left node directly
                self.tree[parent_index] = self.tree[left_index];
            }

            update_queue.push_back(parent_index - leaf_start);
        }

        Some(())
    }
}