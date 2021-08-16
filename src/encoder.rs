mod kt_search_tree;

use bitvec::prelude::*;
use rand::{Rng, SeedableRng};
use rand_pcg::Pcg64;

use self::kt_search_tree::{Config, SearchTree};

pub const BLOCK_SIZE: u8 = 8;
pub const STATE_COUNT: usize = 1 << BLOCK_SIZE;
pub const MASK: usize = STATE_COUNT - 1;
pub type ByteEncodingCapacity = u64;

pub trait KTEncoder {
    fn encode_byte(&self, byte: u8, state: u64) -> (u8, u64);
    fn decode_byte(&self, byte: u8, state: u64) -> (u8, u64);

    fn encode(
        &self,
        message: &Vec<u8>,
        probabilities: &Vec<f32>,
        encoding_capacity: ByteEncodingCapacity,
        freedom_bit_count: u8,
    ) -> Vec<u8>;

    fn decode(
        &self,
        data: &Vec<u8>,
        encoding_capacity: ByteEncodingCapacity,
        freedom_bit_count: u8,
    ) -> Vec<u8>;
}

pub struct StandardKTEncoder {
    transition_function_table: Vec<u64>,
    reverse_transition_function_table: Vec<u64>,
}

fn generate_transition_function_table(seed: u64) -> ([u64; STATE_COUNT], [u64; STATE_COUNT]) {
    let mut rng = Pcg64::seed_from_u64(seed);

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
            transition_function_table[j] =
                transition_function_table[j].wrapping_add(m * temp_values[random_value] as u64);
            if m == 1 {
                reverse_table[temp_values[random_value] as usize] = j as u64;
            }
            temp_values[random_value] = temp_values[STATE_COUNT - j - 1];
        }
        m = m.wrapping_mul(STATE_COUNT as u64);
    }
    (transition_function_table, reverse_table)
}

impl StandardKTEncoder {
    pub fn new(seed: u64) -> Self {
        let (transition_function_table, reverse_transition_function_table) =
            generate_transition_function_table(seed);

        StandardKTEncoder {
            transition_function_table: transition_function_table.into(),
            reverse_transition_function_table: reverse_transition_function_table.into(),
        }
    }
}

impl KTEncoder for StandardKTEncoder {
    fn encode_byte(&self, byte: u8, state: u64) -> (u8, u64) {
        let new_state = state ^ self.transition_function_table[byte as usize];
        (new_state as u8, new_state.rotate_right(BLOCK_SIZE as u32))
    }

    fn decode_byte(&self, byte: u8, state: u64) -> (u8, u64) {
        let index = (byte as u64 ^ state) as usize & MASK;
        let decoded_byte = self.reverse_transition_function_table[index] as u64;
        let new_state = state ^ self.transition_function_table[decoded_byte as usize];
        (
            decoded_byte as u8,
            new_state.rotate_right(BLOCK_SIZE as u32),
        )
    }

    fn encode(
        &self,
        message: &Vec<u8>,
        probabilities: &Vec<f32>,
        encoding_capacity: ByteEncodingCapacity,
        freedom_bit_count: u8,
    ) -> Vec<u8> {
        let config = Config {
            probabilities,
            encoding_capacity,
            freedom_bit_count,
            encoding_method: self,
        };
        let mut search_tree = SearchTree::new(&config);

        search_tree.find_best_encoding(message)
    }

    fn decode(
        &self,
        data: &Vec<u8>,
        encoding_capacity: ByteEncodingCapacity,
        freedom_bit_count: u8,
    ) -> Vec<u8> {
        let bit_capacity = encoding_capacity as usize * 8 ;
        let mut message_bits: BitVec<Lsb0, u8> = bitvec![Lsb0, u8; 0; bit_capacity];

        let mut state = 0;
        let mut bit_position: usize = 0;
        for byte in data {
            let (decoded_byte, new_state) = self.decode_byte(*byte, state);
            state = new_state;

            let decoded_bits = decoded_byte.view_bits::<Lsb0>();
            for i in freedom_bit_count..8 {
                if bit_position >= bit_capacity {
                    break;
                }
                message_bits.set(bit_position, decoded_bits[i as usize]);
                bit_position += 1;
            }
        };

        message_bits.into()
    }
}
