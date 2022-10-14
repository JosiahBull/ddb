use std::{path::Path, process::exit, ops::Range, fs::File, os::unix::prelude::FileExt};

use crate::MIN_BLOCK_SIZE;

pub struct Config {
    pub input_file: String,
    pub output_file: String,
}

struct Block {
    /// Where to start reading from write_job.data
    pub source: Range<usize>,
    /// Where to start writing in the output file
    pub write_offset: u64,
}

pub struct WriteJob {
    pub offset: usize,
    pub data: Vec<u8>,
    blocks: Vec<Block>,
}

impl WriteJob {
    pub fn break_into_blocks(
        mut input: Vec<u8>,
        invalid: &[u8],
        mut limit: usize,
        offset: usize,
    ) -> WriteJob {
        // loop through the input array in MIN_BLOCK_SIZE chunks, if the chunk is invalid, add it's indices into a block
        // if the chunk is valid, drain it's data from the input array, and start reading the next block

        let mut blocks = Vec::new();

        let mut start = offset;
        let mut end = offset + MIN_BLOCK_SIZE;
        loop {
            // if we've reached the end of the input, break
            if end >= limit {
                end = limit;
            }
            if start >= limit {
                break;
            }

            // if the block is invalid, add it to the blocks vector
            if input[start..end] != invalid[start..end] {
                blocks.push(Block {
                    source: start..end,
                    write_offset: start as u64,
                });

                // increment the offset and start/end indices
                start += MIN_BLOCK_SIZE;
                end += MIN_BLOCK_SIZE;

            } else {
                // if the block is valid, drain it from the input array
                input.drain(start..end);

                // decrease the limit by the size of the block
                limit -= MIN_BLOCK_SIZE;
            }
        }

        WriteJob {
            offset,
            data: input,
            blocks,
        }
    }

    pub fn write(self, file: &mut File) -> std::io::Result<usize> {
        let mut written = 0;
        for block in self.blocks.into_iter() {
            let data_slice = &self.data[block.source];
            written += file.write_at(data_slice, block.write_offset)?;
        }
        Ok(written)
    }
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
