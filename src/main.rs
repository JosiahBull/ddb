#[macro_use]
extern crate static_assertions;

use std::{
    fs::OpenOptions,
    io::Read,
    marker::PhantomPinned,
    os::unix::prelude::FileExt,
    pin::Pin,
    ptr::NonNull,
    sync::mpsc::{Receiver, SyncSender},
    thread::sleep,
    time::{Duration, Instant}, process::exit, fmt::Write,
};

use indicatif::{ProgressBar, MultiProgress, ProgressStyle, ProgressState};

const BLOCK_SIZE: usize = 1024 * 1024 * 10; // 1mb
const MIN_BLOCK_SIZE: usize = 512;

const_assert!(BLOCK_SIZE >= MIN_BLOCK_SIZE);
const_assert!(BLOCK_SIZE % MIN_BLOCK_SIZE == 0);
const_assert!(BLOCK_SIZE > 1024 * 512);
const_assert!(BLOCK_SIZE < 1024 * 1024 * 1024);

struct Config {
    input_file: String,
    output_file: String,
}

struct Block<'a> {
    data: &'a [u8],
    offset: u64,
}

struct WriteJob<'a> {
    offset: usize,
    data: [u8; BLOCK_SIZE],
    blocks: Vec<Block<'a>>,
    _pin: PhantomPinned,
}

fn break_into_blocks<'a>(
    input: [u8; BLOCK_SIZE],
    invalid: [u8; BLOCK_SIZE],
    limit: usize,
    offset: usize,
) -> Pin<Box<WriteJob<'a>>> {
    let res = WriteJob {
        offset,
        data: input,
        blocks: Vec::with_capacity(BLOCK_SIZE / MIN_BLOCK_SIZE),
        _pin: PhantomPinned,
    };
    let mut boxed = Box::pin(res);

    // loop through the input array in MIN_BLOCK_SIZE chunks, if the chunk is invalid, add it to the blocks list
    for i in 0..(BLOCK_SIZE / MIN_BLOCK_SIZE) {
        let start = i * MIN_BLOCK_SIZE;
        let mut end = start + MIN_BLOCK_SIZE;
        if end > limit {
            end = limit;
        }

        if start > end {
            break;
        }

        let valid_block = &boxed.data[start..end];
        let potentially_invalid_block = &invalid[start..end];

        if valid_block != potentially_invalid_block {
            let block = Block {
                data: unsafe { NonNull::from(&boxed.data[start..end]).as_ref() },
                offset: (offset + i * MIN_BLOCK_SIZE) as u64,
            };

            unsafe {
                let mut_ref: Pin<&mut WriteJob> = Pin::as_mut(&mut boxed);
                Pin::get_unchecked_mut(mut_ref).blocks.push(block);
            }
        }
    }

    boxed
}

fn reader<'a>(cfg: &'a Config, write_q: SyncSender<Pin<Box<WriteJob<'a>>>>, pb: ProgressBar) {
    // open the input and output files
    let mut i_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.input_file)
        .unwrap();

    let mut o_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.output_file)
        .unwrap();

    // get the size of the file
    let i_file_size = i_file.metadata().unwrap().len();

    pb.set_length(i_file_size);
    pb.set_position(0);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta}) ({msg})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    let mut average = 1;
    let mut read_blocks = 0;
    loop {
        // allocate buffers on the heap
        let mut i_buffer = Box::new([0u8; BLOCK_SIZE]);
        let mut o_buffer = Box::new([0u8; BLOCK_SIZE]);

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
                match o_file.read(&mut *o_buffer) {
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

        if *i_buffer != *o_buffer {
            let boxed =
                break_into_blocks(*i_buffer, *o_buffer, i_bytes_read, read_blocks * BLOCK_SIZE);

            // start timer
            let start = std::time::Instant::now();

            write_q.send(boxed).unwrap();

            let end = std::time::Instant::now();
            let elapsed = end.duration_since(start).as_nanos();

            average -= average / (read_blocks + 1);
            average += elapsed as usize / (read_blocks + 1);
        }

        // if we read less than the block size, we're done
        if i_bytes_read < BLOCK_SIZE || o_bytes_read < BLOCK_SIZE {
            break;
        }
        pb.set_position(((read_blocks+1) * BLOCK_SIZE) as u64);
        pb.set_message(format!("{} blocks/s", 1_000_000_000 / average));

        read_blocks += 1;
    }
    pb.finish_with_message("Done reading");
}

fn writer<'a>(cfg: &'a Config, write_q: Receiver<Pin<Box<WriteJob<'a>>>>, pb: MultiProgress) {
    // open the output file
    let o_file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(false)
        .open(&cfg.output_file)
        .unwrap();

    // wait 100ms
    sleep(Duration::from_millis(100));

    let mut average = 0;
    let mut samples = 0;

    // loop until the write queue is empty
    let mut start = std::time::Instant::now();
    while let Ok(job) = write_q.recv() {
        samples += 1;

        average -= average / samples;
        average += (Instant::now() - start).as_nanos() as u64 / samples;

        // loop through the blocks in the job
        for block in &job.blocks {
            // write the block to the output file
            o_file.write_at(block.data, block.offset).unwrap();
        }

        pb.println(format!("Wrote {} bytes at offset [{}]", job.data.len(), job.offset)).unwrap();

        // start timer
        start = Instant::now();
    }
}

fn controller(cfg: Config) {
    // validate the input and output files exist
    if !std::path::Path::new(&cfg.input_file).exists() {
        eprintln!("Input file does not exist");
        exit(1);
    }
    if !std::path::Path::new(&cfg.output_file).exists() {
        eprintln!("Output file does not exist");
        exit(1);
    }

    let m_pb = MultiProgress::new();

    // create a scoped thread
    std::thread::scope(|scope| {
        let (write_q_tx, write_q_rx) = std::sync::mpsc::sync_channel(100);

        let stack_size = (BLOCK_SIZE*2) + 1024 * 1024;
        let pb = m_pb.add(ProgressBar::new(0));
        let reader_thread = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("reader_thread".to_string())
            .spawn_scoped(scope, || reader(&cfg, write_q_tx, pb)).unwrap();

        let writer_thread = scope.spawn(|| writer(&cfg, write_q_rx, m_pb));

        // wait for the threads to finish
        reader_thread.join().unwrap();
        writer_thread.join().unwrap();
    });
}

fn main() {
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
    controller(config);
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write, BufWriter},
        os::unix::prelude::FileExt, cmp,
    };

    use rand::{Rng, RngCore};

    use crate::{Config, controller};

    #[test]
    fn test_large_file_duplicate() {
        // 256mb
        const FILE_SIZE: usize = 1024 * 1024 * 1024;

        println!("creating random file");
        // write the data to a file
        let file = std::fs::File::create("test_large_file_duplicate.bin").unwrap();

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

        println!("copying random file");

        // copy the file
        std::fs::copy(
            "test_large_file_duplicate.bin",
            "test_large_file_duplicate_copy.bin",
        )
        .unwrap();

        println!("mutating random file");

        // mutate the copy in 400 places, each varying in size from 1 byte to 1mb
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("test_large_file_duplicate_copy.bin")
            .unwrap();

        for i in 0..50 {
            let offset = rng.gen_range(0..FILE_SIZE);
            let size = rng.gen_range(1..(FILE_SIZE - offset).min(1024*5));
            let mut data = vec![0u8; size];
            rng.fill_bytes(&mut data);
            file.write_at(&data, offset as u64).unwrap();

            println!("[{}]: mutated {} bytes at offset [{}/{}]", i, size, offset, FILE_SIZE);
        }

        // mutate the last 5 bytes
        let file_size = file.metadata().unwrap().len();
        file.write_at(&[0, 1, 2, 3, 4], (file_size - 5) as u64).unwrap();

        //mutate the first 5 bytes
        file.write_at(&[5, 6, 7, 8, 9], 0).unwrap();

        println!("running duplicate test");

        let config = Config {
            input_file: "test_large_file_duplicate.bin".to_string(),
            output_file: "test_large_file_duplicate_copy.bin".to_string(),
        };

        // run the controller
        controller(config);

        println!("validating results");

        // validate the files are the same
        let mut file1 = std::fs::File::open("test_large_file_duplicate.bin").unwrap();
        let mut file2 = std::fs::File::open("test_large_file_duplicate_copy.bin").unwrap();

        let mut buffer1 = [0u8; 8128];
        let mut buffer2 = [0u8; 8128];

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
        std::fs::remove_file("test_large_file_duplicate.bin").unwrap();
        std::fs::remove_file("test_large_file_duplicate_copy.bin").unwrap();
    }
}
