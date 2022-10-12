use std::{fs::OpenOptions, io::Read, os::unix::prelude::FileExt};

use crate::{utils::{validate_paths, break_into_blocks, Config}, BLOCK_SIZE};

pub fn controller(cfg: Config) {
    validate_paths(&cfg);

    let mut i_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.input_file)
        .unwrap();

    let mut o_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(&cfg.output_file)
        .unwrap();

    let mut o_buffer = [0u8; 1024*1024];

    let mut read_blocks = 0;
    loop {
        let mut i_buffer = Vec::with_capacity(1024*1024);

        // read from the input and output into the buffer
        let i_bytes_read = {
            loop {
                match i_file.read(&mut *i_buffer) {
                    Ok(n) => break n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => panic!("Error reading from input file: {}", e),
                }
            }
        };
        let o_bytes_read = {
            loop {
                match o_file.read(&mut o_buffer) {
                    Ok(n) => break n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(e) => panic!("Error reading from output file: {}", e),
                }
            }
        };

        // if we read 0 bytes, we're done
        if i_bytes_read == 0 || o_bytes_read == 0 {
            break;
        }

        if i_buffer != o_buffer {
            let boxed =
                break_into_blocks(i_buffer, &o_buffer, i_bytes_read, read_blocks * 1024*1024);

            // loop through the blocks in the job
            for block in &boxed.blocks {
                // write the block to the output file
                // o_file.write_at(block.data, block.offset).unwrap();
            }
        }

        // if we read less than the block size, we're done
        if i_bytes_read < 1024*1024 || o_bytes_read < 1024*1024 {
            break;
        }

        read_blocks += 1;
    }
}
