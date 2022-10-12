use std::{marker::PhantomPinned, pin::Pin, ptr::NonNull, path::Path, process::exit};

use crate::{BLOCK_SIZE, MIN_BLOCK_SIZE};

pub struct Config {
    pub input_file: String,
    pub output_file: String,
}

pub struct Block {
    /// Where to start reading from write_job.data
    pub source_offset: usize,
    /// Where to start writing in the output file
    pub write_offset: usize,
    /// How many bytes to read from write_job.data into the output file
    pub len: u64,
}

pub struct WriteJob {
    pub offset: usize,
    pub data: Vec<u8>,
    pub blocks: Vec<Block>,
}

pub fn break_into_blocks(
    input: Vec<u8>,
    invalid: &[u8],
    limit: usize,
    offset: usize,
) -> WriteJob {
    // loop through the input array in MIN_BLOCK_SIZE chunks, if the chunk is invalid,

    todo!()
}


pub fn validate_paths(cfg: &Config) {
    // check if the input file exists
    if !Path::new(&cfg.input_file).exists() {
        eprintln!("Input file does not exist");
        exit(1);
    }

    // check if the output file exists
    if !Path::new(&cfg.output_file).exists() {
        eprintln!("Output file does not exist");
        exit(1);
    }
}
