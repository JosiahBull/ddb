use std::{fmt::Write, fs::OpenOptions, io::Read};

use indicatif::{ProgressBar, ProgressState, ProgressStyle};

use crate::{
    error::DdsError,
    utils::{validate_paths, WriteJob},
    Dds, BLOCK_SIZE, MIN_BLOCK_SIZE,
};

fn __controller(cfg: Dds) {
    validate_paths(&cfg);

    let mut i_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.input)
        .unwrap();

    let mut o_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(false)
        .open(&cfg.output)
        .unwrap();

    let i_file_size = i_file.metadata().unwrap().len();

    let pb = ProgressBar::new(i_file_size);
    pb.set_position(0);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    let mut o_buffer = [0u8; BLOCK_SIZE];
    let mut read_blocks = 0;
    loop {
        let mut i_buffer = vec![0u8; BLOCK_SIZE];

        // read from the input and output into the buffer
        let i_bytes_read = {
            loop {
                match i_file.read(&mut *i_buffer) {
                    Ok(n) => break n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                        println!("Interrupted");
                        continue;
                    }
                    Err(e) => panic!("Error reading from input file: {}", e),
                }
            }
        };
        let o_bytes_read = {
            loop {
                match o_file.read(&mut o_buffer) {
                    Ok(n) => break n,
                    Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {
                        println!("Interrupted");
                        continue;
                    }
                    Err(e) => panic!("Error reading from output file: {}", e),
                }
            }
        };

        // if we read 0 bytes, we're done
        if i_bytes_read == 0 || o_bytes_read == 0 {
            break;
        }

        if i_buffer != o_buffer {
            let job = WriteJob::break_into_blocks(
                i_buffer.clone(),
                &o_buffer,
                i_bytes_read,
                read_blocks * BLOCK_SIZE,
                MIN_BLOCK_SIZE,
            );
            debug_assert!(!job.is_empty());
            job.write(&mut o_file).unwrap();
        }

        read_blocks += 1;
        pb.set_position((read_blocks * BLOCK_SIZE) as u64);
    }
    pb.finish_with_message("Complete");
}

pub fn controller(cfg: Dds) -> Result<(), DdsError> {
    let stack_size = BLOCK_SIZE + 1024 * 1024;

    let thread = std::thread::Builder::new()
        .name("controller".to_string())
        .stack_size(stack_size)
        .spawn(move || __controller(cfg))
        .unwrap();

    thread.join().unwrap();

    Ok(())
}
