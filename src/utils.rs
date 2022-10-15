use std::{
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::Path,
    process::exit,
};

use crate::Dds;

#[derive(Debug)]
struct Block {
    /// Where to start reading from write_job.data
    pub source: Range<usize>,
    /// Where to start writing in the output file
    pub write_offset: u64,
}

#[derive(Debug)]
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
        min_block_size: usize,
    ) -> WriteJob {
        // loop through the input array in MIN_BLOCK_SIZE chunks, if the chunk is invalid, add it's indices into a block
        // if the chunk is valid, drain it's data from the input array, and start reading the next block

        let mut blocks = Vec::new();

        let mut write_offset = 0;
        let mut start = 0;
        let mut end = min_block_size;
        loop {
            // if we've reached the end of the input, break
            if end >= limit {
                end = limit;
            }
            if start >= end {
                break;
            }

            let step_size = end - start;

            // if the block is invalid, add it to the blocks vector
            if input[start..end] != invalid[(write_offset)..(write_offset + step_size)] {
                blocks.push(Block {
                    source: start..end,
                    write_offset: (write_offset + offset) as u64,
                });

                // increment the offset and start/end indices
                start += step_size;
                end += step_size;
            } else {
                // if the block is valid, drain it from the input array
                input.drain(start..end);

                // decrease the limit by the size of the block
                limit -= step_size;
            }

            write_offset += step_size;
        }

        WriteJob {
            offset,
            data: input,
            blocks,
        }
    }

    pub fn write<T: Seek + Read + Write>(self, file: &mut T) -> std::io::Result<usize> {
        let mut written = 0;
        for block in self.blocks.into_iter() {
            let start_loc = file.stream_position()?;

            // seek and write data into file
            let data_slice = &self.data[block.source];
            file.seek(SeekFrom::Start(block.write_offset))?;
            file.write_all(data_slice)?;
            written += data_slice.len();

            // return cursor to original position
            file.seek(SeekFrom::Start(start_loc))?;
        }
        Ok(written)
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }
}

pub fn validate_paths(cfg: &Dds) {
    // check if the input file exists
    if !Path::new(&cfg.input).exists() {
        eprintln!("Input file does not exist");
        exit(1);
    }

    // check if the output file exists
    if !Path::new(&cfg.output).exists() {
        eprintln!("Output file does not exist");
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[test]
    fn test_break_into_blocks() {
        let input = vec![0u8; 1024 * 1024];
        let invalid = vec![1u8; 1024 * 1024];
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 1024, 0, 1024);
        assert_eq!(job.blocks.len(), 1024);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);
        assert_eq!(written, 1024 * 1024);
    }

    #[test]
    fn test_break_into_blocks_respect_limit() {
        let input = vec![0u8; 1024 * 1024];
        let invalid = vec![1u8; 1024 * 1024];
        let job =
            super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 1024 - 1, 0, 1024);
        assert_eq!(job.blocks.len(), 1024);

        // loop through the blocks and make sure the last block is the correct size
        let last_block = job.blocks.last().unwrap();
        assert_eq!(last_block.source.end - last_block.source.start, 1023);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly, the last byte should be 11
        assert_eq!(c.get_ref()[1024 * 1024 - 1], 1);

        // otherwise all other bytes should be 0
        assert_eq!(c.get_ref()[0..1024 * 1024 - 1], input[0..1024 * 1024 - 1]);

        assert_eq!(written, 1024 * 1024 - 1);
    }

    #[test]
    fn test_break_into_blocks_identical() {
        let input = vec![0u8; 1024 * 1024];
        let invalid = vec![0u8; 1024 * 1024];
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 1024, 0, 1024);
        assert_eq!(job.blocks.len(), 0);
        assert_eq!(job.data.len(), 0);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);

        assert_eq!(written, 0);
    }

    #[test]
    fn test_break_into_blocks_some_identical() {
        let input = vec![0u8; 1024 * 3];
        let mut invalid = vec![0u8; 1024 * 3];
        invalid[1024] = 1;
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 3, 0, 1024);

        assert_eq!(job.blocks.len(), 1);
        assert_eq!(job.data.len(), 1024);

        // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1024);
        assert_eq!(block.write_offset, 1024);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);

        assert_eq!(written, 1024);
    }

    #[test]
    fn test_break_into_blocks_non_divisible_min_block() {
        let input = vec![0u8; 1024 * 3];
        let invalid = vec![1u8; 1024 * 3];
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 3, 0, 1022);

        assert_eq!(job.blocks.len(), 4);
        assert_eq!(job.data.len(), 1024 * 3);

        // // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1022);
        assert_eq!(block.write_offset, 0);

        let block = job.blocks.last().unwrap();
        assert_eq!(block.source.start, 3066);
        assert_eq!(block.source.end, 1024 * 3);
        assert_eq!(block.write_offset, 3066);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);

        assert_eq!(written, 1024 * 3);
    }

    #[test]
    fn test_break_into_blocks_first_byte_invalid() {
        let input = vec![0u8; 1024 * 3];
        let mut invalid = vec![0u8; 1024 * 3];
        invalid[0] = 1;
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 3, 0, 1024);

        assert_eq!(job.blocks.len(), 1);
        assert_eq!(job.data.len(), 1024);

        // // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1024);
        assert_eq!(block.write_offset, 0);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);

        assert_eq!(written, 1024);
    }

    #[test]
    fn test_break_into_blocks_last_byte_invalid() {
        let input = vec![0u8; 1024 * 3];
        let mut invalid = vec![0u8; 1024 * 3];
        invalid[1024 * 3 - 1] = 1;
        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 3, 0, 1024);

        assert_eq!(job.blocks.len(), 1);
        assert_eq!(job.data.len(), 1024);

        // // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1024);
        assert_eq!(block.write_offset, 1024 * 3 - 1024);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);

        // write the data to the fake file
        let written = job.write(&mut c).unwrap();

        // make sure the data was written correctly
        assert_eq!(c.get_ref(), &input);

        assert_eq!(written, 1024);
    }

    #[test]
    fn test_break_into_blocks_large_diff_across_block_boundary() {
        let input = vec![0u8; 1024 * 3];
        let mut invalid = vec![0u8; 1024 * 3];
        for i in invalid.iter_mut().take(1500) {
            *i = 1;
        }

        let job = super::WriteJob::break_into_blocks(input.clone(), &invalid, 1024 * 3, 0, 1024);

        assert_eq!(job.blocks.len(), 2);
        assert_eq!(job.data.len(), 1024 * 2);

        // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1024);
        assert_eq!(block.write_offset, 0);

        let block = job.blocks.last().unwrap();
        assert_eq!(block.source.start, 1024);
        assert_eq!(block.source.end, 1024 * 2);
        assert_eq!(block.write_offset, 1024);

        // Create a fake "file" with the invalid data
        let mut c = Cursor::new(invalid);
        let written = job.write(&mut c).unwrap();
        assert_eq!(c.get_ref(), &input);
        assert_eq!(written, 1024 * 2);
    }

    #[test]
    fn test_break_into_blocks_large_offset() {
        let input = vec![0u8; 1024 * 3];
        let invalid = vec![1u8; 1024 * 3];
        let job = super::WriteJob::break_into_blocks(input, &invalid, 1024 * 3, 1024 * 10, 1024);

        assert_eq!(job.blocks.len(), 3);
        assert_eq!(job.data.len(), 1024 * 3);

        // make sure the adjusted block is correct
        let block = job.blocks.first().unwrap();
        assert_eq!(block.source.start, 0);
        assert_eq!(block.source.end, 1024);
        assert_eq!(block.write_offset, 1024 * 10);

        let block = job.blocks.last().unwrap();
        assert_eq!(block.source.start, 1024 * 2);
        assert_eq!(block.source.end, 1024 * 3);
        assert_eq!(block.write_offset, 1024 * 12);
    }

    #[test]
    fn test_break_into_blocks_real_test() {
        let input = [
            88, 147, 118, 126, 157, 248, 68, 163, 166, 109, 202, 168, 199, 183, 203, 88, 164, 224,
            36, 141, 140, 39, 34, 26, 198, 216, 16, 138, 210, 232, 251, 19, 216, 181, 255, 174,
            204, 77, 74, 93, 93, 178, 202, 75, 182, 39, 125, 149, 139, 189, 205, 222, 81, 165, 242,
            36, 134, 126, 12, 195, 47, 88, 221, 82, 226, 95, 223, 143, 60, 204, 157, 23, 91, 178,
            84, 41, 50, 44, 8, 147, 219, 207, 129, 18, 245, 79, 22, 201, 206, 234, 125, 254, 228,
            74, 127, 160, 175, 60, 87, 16, 149, 85, 159, 251, 123, 69, 158, 62, 206, 216, 54, 153,
            65, 243, 0, 184, 91, 216, 10, 102, 65, 156, 10, 30, 227, 11, 171, 159, 89, 47, 149, 35,
            81, 192, 215, 128, 24, 251, 62, 228, 197, 128, 95, 174, 144, 97, 224, 62, 180, 173,
            175, 236, 162, 155, 180, 119, 67, 158, 149, 169, 90, 163, 41, 52, 111, 212, 41, 60,
            172, 147, 173, 146, 229, 126, 27, 97, 233, 161, 2, 98, 189, 116, 155, 211, 194, 155,
            49, 40, 73, 53, 3, 144, 149, 237, 47, 136, 154, 123, 199, 33, 46, 35, 148, 119, 229,
            237, 101, 121, 130, 165, 236, 240, 12, 39, 85, 178, 227, 218, 66, 57, 93, 40, 2, 60,
            165, 151, 103, 46, 181, 169, 126, 178, 28, 202, 113, 20, 21, 147, 201, 71, 201, 114,
            122, 154, 100, 61, 13, 192, 130, 179, 223, 233, 15, 137, 87, 220, 62, 22, 200, 37, 18,
            46, 124, 152, 68, 154, 34, 196, 227, 34, 198, 144, 151, 62, 159, 181, 130, 164, 124,
            217, 71, 217, 1, 205, 103, 27, 170, 241, 22, 142, 21, 223, 58, 215, 181, 131, 106, 166,
            75, 249, 24, 13, 179, 68, 190, 229, 205, 113, 39, 236, 81, 188, 89, 187, 68, 99, 232,
            158, 167, 50, 102, 53, 198, 237, 165, 110, 37, 249, 92, 190, 37, 243, 175, 74, 95, 95,
            102, 135, 143, 206, 177, 237, 82, 124, 111, 228, 198, 70, 41, 251, 196, 252, 16, 233,
            107, 52, 85, 46, 243, 206, 240, 124, 114, 40, 145, 52, 102, 178, 81, 164, 202, 242,
            119, 73, 48, 72, 246, 180, 11, 92, 110, 91, 160, 136, 3, 238, 5, 167, 56, 224, 38, 77,
            207, 244, 90, 151, 188, 174, 16, 38, 76, 198, 197, 41, 119, 31, 119, 39, 62, 208, 33,
            112, 2, 6, 176, 92, 243, 168, 217, 71, 247, 147, 24, 209, 96, 78, 34, 31, 94, 233, 121,
            69, 185, 71, 155, 123, 229, 213, 70, 183, 83, 95, 43, 12, 229, 167, 180, 189, 81, 211,
            0, 206, 224, 137, 120, 243, 80, 53, 216, 121, 162, 197, 243, 20, 203, 81, 159, 92, 80,
            167, 134, 183, 79, 202, 137, 116, 26, 207, 8, 45, 245, 23, 110, 48, 251, 132, 11, 208,
            140, 171, 163, 137, 119, 169, 134, 254, 141, 7, 135, 164, 110, 218, 212, 186, 187, 188,
            0, 254, 100, 181, 24, 235, 3, 87, 195, 242, 146, 55, 118, 71, 135, 75, 238, 190, 64,
            17, 245, 65, 138, 101, 159, 128, 164, 183, 119, 138, 199, 218, 48, 132, 206, 79, 107,
            249, 226, 239, 158, 80, 25, 64, 211, 149, 206, 131, 213, 133, 20, 47, 36, 73, 172, 189,
            152, 241, 142, 183, 181, 104, 243, 155, 22, 163, 146, 111, 161, 85, 230, 23, 240, 79,
            134, 138, 58, 229, 168, 131, 181, 128, 159, 86, 218, 190, 174, 122, 230, 203, 44, 34,
            143, 17, 131, 234, 103, 182, 133, 176, 249, 199, 10, 149, 152, 234, 69, 158, 122, 162,
            108, 112, 225, 235, 94, 104, 40, 202, 1, 136, 229, 179, 200, 227, 160, 238, 39, 162,
            234, 106, 115, 23, 9, 87, 20, 185, 138, 232, 42, 230, 55, 112, 155, 195, 60, 205, 225,
            141, 159, 222, 54, 53, 12, 146, 219, 170, 223, 90, 190, 99, 229, 144, 101, 228, 198,
            208, 84, 97, 151, 187, 248, 249, 202, 9, 173, 126, 28, 15, 95, 71, 144, 99, 90, 86, 23,
            102, 90, 238, 253, 174, 201, 181, 35, 210, 11, 170, 130, 14, 218, 30, 41, 224, 183, 39,
            43, 67, 60, 106, 125, 46, 1, 198, 80, 234, 6, 111, 7, 124, 102, 10, 146, 130, 222, 110,
            54, 33, 195, 25, 115, 0, 140, 235, 166, 222, 111, 144, 197, 212, 98, 37, 179, 165, 100,
            24, 193, 72, 104, 114, 137, 157, 155, 105, 28, 161, 183, 54, 221, 107, 114, 59, 107, 5,
            92, 67, 238, 41, 201, 207, 51, 156, 203, 168, 127, 195, 179, 248, 195, 245, 251, 3,
            202, 8, 177, 93, 207, 224, 55, 122, 151, 73, 28, 99, 104, 123, 205, 10, 153, 18, 109,
            127, 41, 42, 103, 125, 225, 174, 187, 24, 206, 251, 158, 186, 7, 156, 140, 96, 33, 100,
            215, 133, 236, 78, 127, 178, 91, 138, 8, 229, 14, 160, 7, 207, 181, 243, 177, 56, 123,
            226, 139, 98, 247, 213, 67, 13, 102, 55, 94, 188, 26, 186, 56, 242, 112, 140, 229, 237,
            173, 50, 215, 9, 53, 1, 200, 144, 211, 156, 223, 229, 96, 15, 183, 67, 246, 134, 115,
            147, 148, 250, 114, 129, 77, 238, 253, 34, 241, 237, 177, 24, 196, 85, 23, 167, 144,
            182, 11, 13, 169, 241, 125, 204, 41, 65, 34, 98, 73, 123, 179, 16, 211, 60, 236, 246,
            121, 243, 75, 24, 245, 41, 61, 135, 112, 196, 143, 222, 10, 83, 203, 23, 253, 135, 48,
            78, 106, 177, 21, 128, 4, 108, 73, 224, 153, 175, 255, 93, 117, 130, 11, 254, 32, 217,
            151, 54, 204, 221, 57, 18, 67, 116, 52, 227, 7, 67, 57, 40, 97, 107, 179, 199, 240, 87,
            25, 247, 166, 243, 205, 147, 250, 158, 24, 168, 211, 146, 167, 98, 221, 79, 167, 67, 7,
            172, 197, 86, 8, 204, 2, 115, 119, 140, 18, 40, 105, 190, 96, 114, 76, 164, 120, 140,
            254, 146, 18, 58, 70, 13, 155, 53, 69, 144,
        ];
        let invalid = [
            88, 147, 118, 126, 157, 248, 68, 163, 166, 109, 202, 168, 199, 183, 203, 88, 164, 224,
            36, 141, 140, 39, 34, 26, 198, 216, 16, 138, 210, 232, 251, 19, 216, 181, 255, 174,
            204, 77, 74, 93, 93, 178, 202, 75, 182, 39, 125, 149, 139, 189, 205, 222, 81, 165, 242,
            36, 134, 126, 12, 195, 47, 88, 221, 82, 226, 95, 223, 143, 60, 204, 157, 23, 91, 178,
            84, 41, 50, 44, 8, 147, 219, 207, 129, 18, 245, 79, 22, 201, 206, 234, 125, 254, 228,
            74, 127, 160, 175, 60, 87, 16, 149, 85, 159, 251, 123, 69, 158, 62, 206, 216, 54, 153,
            65, 243, 0, 184, 91, 216, 10, 102, 65, 156, 10, 30, 227, 11, 171, 159, 89, 47, 149, 35,
            81, 192, 215, 128, 24, 251, 62, 228, 197, 128, 95, 174, 144, 97, 224, 62, 180, 173,
            175, 236, 162, 155, 180, 119, 67, 158, 149, 169, 90, 163, 41, 52, 111, 212, 41, 60,
            172, 147, 173, 146, 229, 126, 27, 97, 133, 103, 34, 203, 41, 32, 160, 148, 60, 188,
            189, 111, 92, 119, 81, 240, 113, 175, 23, 8, 101, 135, 98, 80, 151, 11, 178, 237, 33,
            197, 176, 18, 79, 64, 183, 237, 98, 27, 107, 102, 95, 60, 3, 1, 253, 189, 74, 166, 193,
            43, 105, 113, 144, 142, 16, 35, 236, 217, 59, 214, 56, 245, 223, 189, 220, 231, 180,
            164, 7, 137, 71, 57, 84, 144, 103, 181, 137, 216, 159, 72, 143, 181, 12, 39, 209, 18,
            101, 171, 219, 138, 167, 51, 203, 58, 194, 237, 119, 199, 150, 252, 71, 60, 207, 29,
            72, 201, 229, 85, 160, 238, 77, 102, 248, 194, 244, 64, 87, 73, 103, 53, 179, 67, 234,
            208, 110, 195, 7, 212, 95, 214, 47, 60, 243, 83, 168, 44, 165, 150, 56, 155, 37, 94, 0,
            113, 158, 188, 135, 142, 197, 138, 5, 166, 30, 140, 167, 248, 188, 227, 170, 214, 101,
            201, 122, 79, 115, 239, 54, 183, 216, 202, 113, 214, 42, 169, 57, 240, 136, 235, 114,
            216, 44, 111, 28, 221, 206, 36, 242, 75, 90, 224, 168, 80, 131, 172, 229, 104, 194,
            191, 3, 170, 8, 45, 22, 121, 215, 148, 102, 12, 220, 180, 14, 196, 61, 205, 211, 43,
            86, 175, 207, 0, 239, 169, 147, 241, 171, 112, 206, 36, 176, 34, 90, 101, 140, 110,
            103, 123, 249, 12, 172, 31, 186, 73, 72, 251, 9, 95, 136, 46, 144, 154, 98, 22, 49, 75,
            112, 132, 39, 24, 119, 171, 188, 164, 253, 94, 49, 200, 152, 124, 163, 222, 39, 211,
            255, 148, 2, 205, 232, 236, 126, 246, 57, 3, 95, 125, 142, 179, 125, 98, 176, 134, 38,
            77, 219, 140, 158, 156, 60, 114, 163, 42, 218, 108, 56, 28, 228, 107, 27, 114, 76, 185,
            239, 223, 247, 13, 160, 254, 234, 239, 242, 211, 40, 26, 62, 84, 107, 72, 66, 105, 116,
            206, 116, 14, 253, 238, 212, 213, 76, 86, 177, 221, 166, 191, 214, 107, 141, 102, 31,
            182, 217, 169, 237, 107, 169, 210, 244, 224, 92, 80, 34, 118, 210, 68, 167, 191, 138,
            11, 242, 214, 167, 34, 28, 53, 121, 254, 54, 181, 149, 124, 180, 193, 244, 133, 118,
            114, 10, 110, 0, 102, 249, 56, 197, 3, 176, 115, 45, 224, 200, 234, 231, 187, 174, 60,
            193, 85, 169, 119, 169, 46, 39, 73, 2, 58, 170, 51, 41, 234, 187, 227, 235, 219, 96,
            196, 13, 255, 224, 101, 229, 146, 46, 209, 47, 76, 84, 115, 53, 168, 165, 170, 248,
            185, 40, 82, 216, 159, 209, 110, 88, 115, 202, 216, 245, 90, 227, 138, 113, 119, 252,
            26, 127, 221, 15, 136, 3, 161, 162, 22, 130, 193, 44, 96, 202, 147, 10, 247, 75, 36,
            123, 106, 138, 6, 199, 191, 194, 30, 233, 59, 23, 6, 214, 170, 196, 49, 94, 70, 162,
            40, 146, 28, 194, 12, 164, 101, 182, 77, 135, 69, 200, 31, 26, 59, 15, 238, 140, 134,
            237, 97, 20, 41, 226, 112, 236, 92, 31, 60, 205, 14, 48, 183, 73, 60, 47, 190, 98, 19,
            115, 103, 52, 63, 180, 60, 220, 247, 76, 237, 23, 23, 17, 27, 184, 171, 243, 49, 66,
            255, 104, 140, 245, 158, 155, 145, 64, 55, 39, 195, 27, 91, 33, 111, 217, 138, 169,
            140, 236, 15, 164, 223, 7, 116, 90, 75, 185, 96, 125, 58, 253, 71, 53, 222, 209, 174,
            7, 242, 234, 28, 134, 80, 26, 54, 210, 148, 245, 13, 198, 112, 44, 88, 100, 196, 242,
            137, 124, 186, 196, 28, 154, 176, 180, 239, 25, 86, 249, 131, 34, 90, 123, 221, 149,
            204, 153, 49, 48, 247, 241, 244, 187, 181, 91, 0, 13, 247, 26, 80, 103, 68, 162, 221,
            45, 242, 6, 18, 204, 213, 169, 108, 53, 106, 106, 159, 46, 147, 141, 186, 156, 215,
            206, 5, 121, 204, 42, 206, 219, 77, 181, 132, 10, 54, 122, 139, 25, 52, 3, 66, 171, 12,
            51, 170, 112, 169, 96, 254, 166, 181, 133, 120, 194, 148, 218, 176, 60, 222, 132, 148,
            70, 188, 46, 101, 80, 188, 150, 87, 103, 122, 2, 5, 143, 219, 172, 83, 207, 193, 154,
            193, 39, 50, 107, 140, 87, 179, 108, 203, 237, 126, 191, 81, 157, 17, 112, 133, 12,
            217, 172, 105, 235, 191, 65, 56, 250, 61, 84, 184, 54, 54, 183, 167, 190, 1, 91, 93,
            201, 136, 58, 33, 26, 144, 2, 196, 165, 144, 39, 149, 57, 144, 42, 179, 86, 11, 254,
            153, 13, 39, 144, 236, 122, 183, 72, 107, 15, 35, 6, 3, 138, 43, 116, 112, 54, 163,
            166, 127, 17, 151, 25, 242, 236, 125, 113, 195, 78, 108, 6, 96, 94, 27, 4, 50, 95, 39,
            37, 34, 45, 186, 187, 51, 3, 147, 6, 222, 43, 219, 190, 43, 156, 188, 255, 234, 12,
            144, 197, 46, 122, 138, 37, 66, 8, 149, 115, 200, 230, 127,
        ];

        let input_vec = Vec::from(input);
        let job = super::WriteJob::break_into_blocks(input_vec, &invalid, 1024, 3042304, 512);

        assert_eq!(job.len(), 2);
        assert_eq!(job.blocks[0].write_offset, 3042304);
        assert_eq!(job.blocks[0].source, 0..512);
        assert_eq!(job.blocks[1].write_offset, 3042304 + 512);
        assert_eq!(job.blocks[1].source, 512..1024);
    }
}
