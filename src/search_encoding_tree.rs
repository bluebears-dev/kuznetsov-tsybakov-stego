use std::usize;

use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

pub const BLOCK_SIZE: u8 = 8;
pub const STATE_COUNT: usize = 1 << BLOCK_SIZE;
pub const MASK: usize = STATE_COUNT - 1;
const RNG_SEED: u64 = 1;


pub fn generate_transition_function_table() -> ([u64; STATE_COUNT], [u64; STATE_COUNT]) {
    let mut rng = Pcg64::seed_from_u64(RNG_SEED);

    let mut m: u64 = 1;
    let mut transition_function_table: [u64; STATE_COUNT] = [0; STATE_COUNT];
    let mut reverse_table: [u64; STATE_COUNT] = [0; STATE_COUNT];

    let mut temp_values: [u64; STATE_COUNT] = [0; STATE_COUNT];
    for _ in 0..(64 / BLOCK_SIZE) {
        for j in 0..STATE_COUNT {
            temp_values[j] = j as u64;
        }

        for j in 0..STATE_COUNT {
            let random_value: usize = rng.gen::<usize>() % (STATE_COUNT - j);
            transition_function_table[j] = transition_function_table[j].wrapping_add(m * temp_values[random_value] as u64);
            if m == 1 {
                reverse_table[temp_values[random_value] as usize] = j as u64;
            }
            temp_values[random_value] = temp_values[STATE_COUNT - j - 1];
        }
        m = m.wrapping_mul(STATE_COUNT as u64);
    }
    (transition_function_table, reverse_table)
}

pub struct SearchHistory {
    nodes: Vec<SearchNode>,
    weights: Vec<f32>,
    transition_fun_table: [u64; STATE_COUNT],
    rev_transition_fun_table: [u64; STATE_COUNT],
    selected_node: usize,
}

impl SearchHistory {
    pub fn new() -> Self {
        let (trans_table, rev_trans_table) = generate_transition_function_table();
        SearchHistory {
            nodes: vec![],
            weights: vec![],
            transition_fun_table: trans_table,
            rev_transition_fun_table: rev_trans_table,
            selected_node: 0,
        }
    }

    pub fn get(&self, index: usize) -> &SearchNode {
        &self.nodes[index]
    }

    pub fn set_weights(&mut self, weights: Vec<f32>) {
        self.weights = weights;
    }

    fn get_encoding_node(&self, node: &SearchNode, byte: u8) -> SearchNode {
        let (new_state, encoded_byte) = encode((node.state, byte), &self.transition_fun_table);
        
        SearchNode::new(
            Some(self.selected_node),
            encoded_byte,
            new_state,
            node.weight + self.weights[encoded_byte as usize],
        )
    }

    pub fn expand_search(
        &mut self,
        prev_nodes: &Vec<SearchNode>,
        max_weight: f32,
        freedom_bit_count: u8,
        bits_to_encode: u8,
    ) -> Vec<SearchNode> {
        let mut new_nodes = vec![];

        for prev_node in prev_nodes {
            // Expand most promising nodes
            if prev_node.weight <= max_weight {
                let saved_node =
                    SearchNode::new(prev_node.parent_index, prev_node.encoded_byte, 0, 0.0_0f32);
                self.nodes.push(saved_node);

                for freedom_bits_value in 0..u8::pow(2, freedom_bit_count.into()) {
                    let modified_byte = bits_to_encode + freedom_bits_value;
                    new_nodes.push(self.get_encoding_node(prev_node, modified_byte));
                }
                self.selected_node += 1;
            }
        }
        new_nodes
    }
}

fn encode((state, byte): (u64, u8), transition_fun_table: &[u64; STATE_COUNT]) -> (u64, u8) {
    let new_state = state ^ transition_fun_table[byte as usize];
    (
        new_state.rotate_right(BLOCK_SIZE as u32),
        new_state as u8,
    )
}

#[derive(Debug)]
pub struct SearchNode {
    pub parent_index: Option<usize>,
    pub encoded_byte: u8,
    state: u64,
    pub weight: f32,
}

impl SearchNode {
    pub fn new(parent_index: Option<usize>, encoded_byte: u8, state: u64, weight: f32) -> Self {
        SearchNode {
            parent_index: parent_index,
            encoded_byte,
            state,
            weight,
        }
    }
}
