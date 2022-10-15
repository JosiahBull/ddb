use std::{
    cmp,
    io::{BufWriter, Write},
    os::unix::prelude::FileExt,
};

use rand::{Rng, RngCore};

// 1024mb
const FILE_SIZE: usize = 1024 * 1024 * 1024;

pub fn generate_test_file(filename: &str) {
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
            let buffer = &mut buffer[..to_write];
            rng.fill(buffer);
            buf_writer.write_all(buffer).unwrap();

            remaining_size -= to_write;
        }

        buf_writer.flush().unwrap();

        println!("copying random file");

        // copy the file
        std::fs::copy(filename, format!("{}.copy", filename)).unwrap();

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
            let offset = rng.gen_range(0..FILE_SIZE - 1);
            let size = rng.gen_range(1..(FILE_SIZE - offset).min(1024 * 5));
            let mut data = vec![0u8; size];
            rng.fill_bytes(&mut data);
            file.write_at(&data, offset as u64).unwrap();

            println!(
                "[{}]: mutated {} bytes at offset [{}/{}]",
                i, size, offset, FILE_SIZE
            );
            total_written += size;
        }

        // mutate the last 5 bytes
        let file_size = file.metadata().unwrap().len();
        file.write_at(&[0, 1, 2, 3, 4], (file_size - 5) as u64)
            .unwrap();

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
