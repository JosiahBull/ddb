mod common;

use std::io::Read;

use assert_cmd::Command;
use dds::{single::controller as single_threaded_controller, Dds};
use sha2::{Digest, Sha256};

use crate::common::generate_test_file;

#[test]
fn test_large_file_duplicate_single() {
    generate_test_file("test_large_file_duplicate-single.bin");

    println!("running duplicate test");

    let config = Dds {
        input: "test_large_file_duplicate-single.bin".to_string(),
        output: "test_large_file_duplicate-single.bin.copy".to_string(),
        threaded: false,
        generate: None,
    };

    // run the controller
    single_threaded_controller(config).unwrap();

    println!("validating results");

    // validate the files are the same
    let mut file1 = std::fs::File::open("test_large_file_duplicate-single.bin").unwrap();
    let mut file2 = std::fs::File::open("test_large_file_duplicate-single.bin.copy").unwrap();

    const COMP_STEP_SIZE: usize = 1024 * 4;
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

#[test]
fn test_singlethreaded_cli() {
    generate_test_file("test_single_cli.bin");

    // hash the source file
    let start_hash: String = {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open("test_single_cli.bin").unwrap();
        std::io::copy(&mut file, &mut hasher).unwrap();
        format!("{:x}", hasher.finalize())
    };

    Command::cargo_bin("dds")
        .unwrap()
        .arg("--input")
        .arg("test_single_cli.bin")
        .arg("--output")
        .arg("test_single_cli.bin.copy")
        .arg("--threaded")
        .write_stdin("y\n")
        .assert()
        .success();

    // validate the files are the same
    let mut file1 = std::fs::File::open("test_single_cli.bin").unwrap();
    let mut file2 = std::fs::File::open("test_single_cli.bin.copy").unwrap();

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

    // validate that the hash of the hash is still the same as it was initially
    let end_hash: String = {
        let mut hasher = Sha256::new();
        let mut file = std::fs::File::open("test_single_cli.bin.copy").unwrap();
        std::io::copy(&mut file, &mut hasher).unwrap();
        format!("{:x}", hasher.finalize())
    };
    assert_eq!(start_hash, end_hash);

    // remove the files
    std::fs::remove_file("test_single_cli.bin").unwrap();
    std::fs::remove_file("test_single_cli.bin.copy").unwrap();
}
