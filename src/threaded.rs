use std::{
    fmt::Write,
    fs::OpenOptions,
    io::Read,
    sync::mpsc::{Receiver, SyncSender},
    time::Instant,
};

use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};

use crate::{
    error::DdsError,
    utils::{validate_paths, WriteJob},
    Dds, BLOCK_SIZE, MIN_BLOCK_SIZE,
};

fn reader(cfg: &Dds, write_q: SyncSender<WriteJob>, pb: ProgressBar) {
    // open the input and output files
    let mut i_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.input)
        .unwrap();

    let mut o_file = OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(&cfg.output)
        .unwrap();

    // get the size of the file
    let i_file_size = i_file.metadata().unwrap().len();

    pb.set_length(i_file_size);
    pb.set_position(0);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));

    let mut read_blocks = 0;
    let mut o_buffer = [0u8; BLOCK_SIZE];
    loop {
        // allocate buffers on the heap
        let mut i_buffer = vec![0u8; BLOCK_SIZE];

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
            let job = WriteJob::break_into_blocks(
                i_buffer,
                &o_buffer,
                i_bytes_read,
                read_blocks * BLOCK_SIZE,
                MIN_BLOCK_SIZE,
            );
            debug_assert!(!job.is_empty());
            write_q.send(job).unwrap();
        }

        pb.set_position(((read_blocks + 1) * BLOCK_SIZE) as u64);
        read_blocks += 1;
    }
    pb.finish_with_message("Complete");
}

fn writer(cfg: &Dds, write_q: Receiver<WriteJob>, pb: MultiProgress) {
    // open the output file
    let mut o_file = OpenOptions::new()
        .read(false)
        .write(true)
        .create(false)
        .open(&cfg.output)
        .unwrap();

    let mut average = 0;
    let mut samples = 0;

    // loop until the write queue is empty
    let mut start = std::time::Instant::now();
    while let Ok(job) = write_q.recv() {
        samples += 1;

        average -= average / samples;
        average += (Instant::now() - start).as_nanos() as u64 / samples;

        pb.println(format!(
            "Wrote {} bytes at offset [{}]",
            job.data.len(),
            &job.offset
        ))
        .unwrap();

        job.write(&mut o_file).unwrap();

        // start timer
        start = Instant::now();
    }
}

pub fn controller(cfg: Dds) -> Result<(), DdsError> {
    validate_paths(&cfg);

    let m_pb = MultiProgress::new();

    // create a scoped thread
    std::thread::scope(|scope| {
        let (write_q_tx, write_q_rx) = std::sync::mpsc::sync_channel(100);

        let stack_size = BLOCK_SIZE + 1024 * 1024;
        let pb = m_pb.add(ProgressBar::new(0));
        let reader_thread = std::thread::Builder::new()
            .stack_size(stack_size)
            .name("reader_thread".to_string())
            .spawn_scoped(scope, || reader(&cfg, write_q_tx, pb))
            .unwrap();

        let writer_thread = scope.spawn(|| writer(&cfg, write_q_rx, m_pb));

        // wait for the threads to finish
        reader_thread.join().unwrap();
        writer_thread.join().unwrap();
    });

    Ok(())
}
