use std::usize;

use bit_vec::{BitVec, Iter};

use crate::encoder::KTEncoder;

use super::ByteEncodingCapacity;

pub struct Config<'a> {
    pub freedom_bit_count: u8,
    pub probabilities: &'a Vec<f32>,
    pub encoding_capacity: ByteEncodingCapacity,
    pub encoding_method: &'a dyn KTEncoder,
}

pub struct SearchTree<'a> {
    search_history: Vec<SearchNode>,
    config: &'a Config<'a>,
    search_index: usize,
}

impl<'a> SearchTree<'a> {
    pub fn new(config: &'a Config) -> Self {
        SearchTree {
            search_history: vec![],
            search_index: 0,
            config,
        }
    }

    pub fn find_best_encoding(&mut self, message: &Vec<u8>) -> Vec<u8> {
        let msg_bit_vec = BitVec::from_bytes(message);
        let maybe_last_node = self.construct_search_tree_approx(&mut msg_bit_vec.iter());

        let mut encoded_message: Vec<u8> = vec![];
        if let Some(last_node) = maybe_last_node {
            encoded_message.push(last_node.encoded_byte);
            println!("{:?}", last_node);
            let mut maybe_node = Some(&last_node);

            for _ in 1..self.config.encoding_capacity {
                maybe_node = maybe_node
                    .map(|node| node.parent_index)
                    .flatten()
                    .map(|index| self.search_history.get(index))
                    .flatten();

                if let Some(node) = maybe_node {
                    encoded_message.push(node.encoded_byte);
                } else {
                    break;
                }
            }
            encoded_message.reverse();
        }
        println!("{:?}", encoded_message.len());
        encoded_message
    }

    fn construct_search_tree_approx(&mut self, msg_bit_iterator: &mut Iter) -> Option<SearchNode> {
        let encoding_capacity = self.config.encoding_capacity;
        let freedom_bit_count = self.config.freedom_bit_count;

        let mut prev_search = vec![SearchNode::new(None, 0, 0, 0.0_f32)];

        let mut max_weight = 1.0;
        let mut min_weight = 0.0;

        for step in 0..encoding_capacity {
            let byte_weights =
                calculate_byte_value_weights_for_step(&self.config.probabilities, step as usize);

            // Store `8 - freedom_bit_count` in MS positions in `bits_to_encode`
            let mut bits_to_encode = 0;
            for i in (freedom_bit_count..8).rev() {
                if let Some(bit) = msg_bit_iterator.next().map(|val| (val as u8) << i) {
                    bits_to_encode += bit;
                }
            }
            prev_search = self.expand_tree(&prev_search, &byte_weights, max_weight, bits_to_encode);

            let (new_max_weight, new_min_weight) = calculate_minmax_weight(&prev_search);
            max_weight = new_max_weight;
            min_weight = new_min_weight;
        }

        prev_search
            .iter()
            .find(|node| node.weight == min_weight)
            .map(|last_node| {
                SearchNode::new(last_node.parent_index, last_node.encoded_byte, 0, 0.0)
            })
    }

    fn expand_tree(
        &mut self,
        prev_nodes: &Vec<SearchNode>,
        weights: &ByteValueWeightArray,
        max_weight: f32,
        byte: u8,
    ) -> Vec<SearchNode> {
        let mut new_nodes = vec![];

        for prev_node in prev_nodes {
            // Expand most promising nodes
            if prev_node.weight <= max_weight {
                let saved_node =
                    SearchNode::new(prev_node.parent_index, prev_node.encoded_byte, 0, 0.0_0f32);
                self.search_history.push(saved_node);

                for freedom_bits_value in 0..u8::pow(2, self.config.freedom_bit_count.into()) {
                    let modified_byte = byte + freedom_bits_value;
                    new_nodes.push(self.get_encoding_node(prev_node, weights, modified_byte));
                }
                self.search_index += 1;
            }
        }
        new_nodes
    }

    fn get_encoding_node(
        &self,
        node: &SearchNode,
        weights: &ByteValueWeightArray,
        byte: u8,
    ) -> SearchNode {
        let (encoded_byte, new_state) = self.config.encoding_method.encode_byte(byte, node.state);

        SearchNode::new(
            Some(self.search_index),
            encoded_byte,
            new_state,
            node.weight + weights[encoded_byte as usize],
        )
    }
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

type ByteValueWeightArray = [f32; 256];
type BitValueWeightArray = [f32; 8];

fn calculate_bit_weights_for_step(
    probabilities: &Vec<f32>,
    step: usize,
) -> (BitValueWeightArray, BitValueWeightArray) {
    let mut zero_bit_weights: BitValueWeightArray = [0.0_f32; 8];
    let mut one_bit_weights: BitValueWeightArray = [0.0_f32; 8];
    for i in 0..8 {
        zero_bit_weights[i] = -(1.0 - probabilities[8 * step + i]).ln();
        one_bit_weights[i] = -(probabilities[8 * step + i]).ln();
    }

    (zero_bit_weights, one_bit_weights)
}

fn calculate_byte_value_weights_for_step(
    probabilities: &Vec<f32>,
    step: usize,
) -> ByteValueWeightArray {
    let weights = calculate_bit_weights_for_step(&probabilities, step as usize);
    let mut all_byte_weights: ByteValueWeightArray = [0.0_f32; 256];

    for byte in 0..256 {
        all_byte_weights[byte] = (0..8)
            .into_iter()
            .map(|index| {
                if (byte & (1 << index)) == 0 {
                    weights.0[index]
                } else {
                    weights.1[index]
                }
            })
            .sum::<f32>();
    }
    all_byte_weights
}

fn calculate_minmax_weight(prev_search: &Vec<SearchNode>) -> (f32, f32) {
    let weights: Vec<f32> = prev_search.iter().map(|node| node.weight).collect();
    let mut max_weight = weights.iter().cloned().fold(0. / 0., f32::max);
    let min_weight = weights.iter().cloned().fold(0. / 0., f32::min);

    // TODO: is the maxactive >> F needed?
    // mxactive >> freedom_bits_count
    if prev_search.len() > (10_000 >> 2) {
        let interpolation_coeff = 1000.0 / (max_weight - min_weight);
        let mut bucket = [0; 1001];
        for node in prev_search {
            let interpolated_index =
                (interpolation_coeff * (node.weight - min_weight)).floor() as usize;
            bucket[interpolated_index] += 1;
        }
        let mut pred = 0;
        let mut i: i32 = 0;
        loop {
            if i >= 1000 || pred >= (10_000 >> 2) {
                break;
            }
            pred += bucket[i as usize];
            i += 1;
        }
        max_weight = min_weight + (i - 2) as f32 * (max_weight - min_weight) / 1000.0;
    }

    (max_weight, min_weight)
}
