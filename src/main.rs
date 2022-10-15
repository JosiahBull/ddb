#[macro_use]
extern crate static_assertions;

use std::process::exit;
use human_panic::setup_panic;

use crate::utils::Config;

mod error;
mod utils;
mod single;
mod threaded;

const BLOCK_SIZE: usize = 1024 * 1024;
const MIN_BLOCK_SIZE: usize = 512;

const_assert!(BLOCK_SIZE >= MIN_BLOCK_SIZE);
const_assert!(BLOCK_SIZE % MIN_BLOCK_SIZE == 0);
const_assert!(BLOCK_SIZE % 2 == 0);
const_assert!(BLOCK_SIZE > 1024 * 512);
const_assert!(BLOCK_SIZE < 1024 * 1024 * 1024);

fn main() {
    setup_panic!();

    // read the first arg (input) and second arg (output)
    let in_file = std::env::args().nth(1).unwrap();
    let out_file = std::env::args().nth(2).unwrap();

    println!("Are you sure you want to overwrite {}? (y/n)", &out_file);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    if input.trim() != "y" {
        eprintln!("Aborting");
        exit(1);
    }

    // create the config
    let config = Config {
        input_file: in_file,
        output_file: out_file,
    };

    // run the controller
    threaded::controller(config);
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write, BufWriter},
        os::unix::prelude::FileExt, cmp,
    };

    use rand::{Rng, RngCore};

    use crate::{Config, threaded::controller as multi_threaded_controller, single::controller as single_threaded_controller};

    // 1024mb
    const FILE_SIZE: usize = 1024 * 1024 * 1024;

    fn generate_test_file(filename: &str) {
        {
            println!("creating random file");
            // write the data to a file
            let file = std::fs::File::create(filename).unwrap();

            let mut buf_writer = BufWriter::new(file);
            let mut rng = rand::thread_rng();
            let mut buffer = [0; 1024];
            let mut remaining_size = FILE_SIZE;

            while remaining_size > 0 {
                let to_write = cmp::min(remaining_size, buffer.len());
                let buffer=  &mut buffer[..to_write];
                rng.fill(buffer);
                buf_writer.write_all(buffer).unwrap();

                remaining_size -= to_write;
            }

            buf_writer.flush().unwrap();

            println!("copying random file");

            // copy the file
            std::fs::copy(
                filename,
                format!("{}.copy", filename),
            )
            .unwrap();

            println!("mutating random file");

            // mutate the copy in 400 places, each varying in size from 1 byte to 1mb
            let file = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .append(false)
                .open(format!("{}.copy", filename))
                .unwrap();

            let mut total_written = 0;

            for i in 0..50 {
                let offset = rng.gen_range(0..FILE_SIZE-1);
                let size = rng.gen_range(1..(FILE_SIZE - offset).min(1024*5));
                let mut data = vec![0u8; size];
                rng.fill_bytes(&mut data);
                file.write_at(&data, offset as u64).unwrap();

                println!("[{}]: mutated {} bytes at offset [{}/{}]", i, size, offset, FILE_SIZE);
                total_written += size;
            }

            // mutate the last 5 bytes
            let file_size = file.metadata().unwrap().len();
            file.write_at(&[0, 1, 2, 3, 4], (file_size - 5) as u64).unwrap();

            //mutate the first 5 bytes
            file.write_at(&[5, 6, 7, 8, 9], 0).unwrap();

            println!("total written: {}", total_written);
        }

        // validate the files are the same size
        let file1 = std::fs::File::open(filename).unwrap();
        let file2 = std::fs::File::open(format!("{}.copy", filename)).unwrap();

        // validate the files are the same size
        let file1_size = file1.metadata().unwrap().len();
        let file2_size = file2.metadata().unwrap().len();

        println!("Size diff {}", file1_size - file2_size);

        assert_eq!(file1_size, file2_size);
    }

    #[test]
    fn test_large_file_duplicate_mutli() {
        generate_test_file("test_large_file_duplicate-multi.bin");

        println!("running duplicate test");

        let config = Config {
            input_file: "test_large_file_duplicate-multi.bin".to_string(),
            output_file: "test_large_file_duplicate-multi.bin.copy".to_string(),
        };

        // run the controller
        multi_threaded_controller(config);

        println!("validating results");

        // validate the files are the same
        let mut file1 = std::fs::File::open("test_large_file_duplicate-multi.bin").unwrap();
        let mut file2 = std::fs::File::open("test_large_file_duplicate-multi.bin.copy").unwrap();

        // validate the files are the same size
        let file1_size = file1.metadata().unwrap().len();
        let file2_size = file2.metadata().unwrap().len();
        assert_eq!(file1_size, file2_size);

        const COMP_STEP_SIZE: usize = 8192;
        let mut buffer1 = [0u8; COMP_STEP_SIZE];
        let mut buffer2 = [0u8; COMP_STEP_SIZE];

        loop {
            let bytes_read1 = file1.read(&mut buffer1).unwrap();
            let bytes_read2 = file2.read(&mut buffer2).unwrap();

            if bytes_read1 == 0 || bytes_read2 == 0 {
                break;
            }

            assert_eq!(bytes_read1, bytes_read2);
            assert_eq!(buffer1, buffer2);
        }

        // remove the files
        std::fs::remove_file("test_large_file_duplicate-multi.bin").unwrap();
        std::fs::remove_file("test_large_file_duplicate-multi.bin.copy").unwrap();
    }

    #[test]
    fn test_large_file_duplicate_single() {
        generate_test_file("test_large_file_duplicate-single.bin");

        println!("running duplicate test");

        let config = Config {
            input_file: "test_large_file_duplicate-single.bin".to_string(),
            output_file: "test_large_file_duplicate-single.bin.copy".to_string(),
        };

        // run the controller
        single_threaded_controller(config);

        println!("validating results");

        // validate the files are the same
        let mut file1 = std::fs::File::open("test_large_file_duplicate-single.bin").unwrap();
        let mut file2 = std::fs::File::open("test_large_file_duplicate-single.bin.copy").unwrap();

        const COMP_STEP_SIZE: usize = 1024*4;
        let mut buffer1 = [0u8; COMP_STEP_SIZE];
        let mut buffer2 = [0u8; COMP_STEP_SIZE];

        loop {
            let bytes_read1 = file1.read(&mut buffer1).unwrap();
            let bytes_read2 = file2.read(&mut buffer2).unwrap();

            if bytes_read1 == 0 || bytes_read2 == 0 {
                break;
            }

            assert_eq!(bytes_read1, bytes_read2);
            assert_eq!(buffer1, buffer2);
        }

        // remove the files
        std::fs::remove_file("test_large_file_duplicate-single.bin").unwrap();
        std::fs::remove_file("test_large_file_duplicate-single.bin.copy").unwrap();
    }
}
