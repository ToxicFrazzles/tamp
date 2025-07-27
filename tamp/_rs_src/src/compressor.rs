use std::collections::VecDeque;
use std::fs::File;
use std::io::{Write, Result, BufWriter};
use crate::bit_writer::BitWriter;
use crate::ring_buffer::RingBuffer;
use crate::common::{tamp_initialize_dictionary, tamp_compute_min_pattern_size};

pub struct Compressor<W: Write> {
    bit_writer: BitWriter<W>,
    window_bits: u8,
    literal_bits: u8,
    min_pattern_size: usize,
    max_pattern_size: usize,
    literal_flag: u16,
    window_buffer: RingBuffer,
    input_buffer: VecDeque<u8>,
}

impl<W: Write> Compressor<W> {
    pub fn new(mut writer: W, window: u8, literal: u8, mut dictionary: Option<Vec<u8>>) -> Result<Self> {
        // Validate window/literal
        assert!((8..=15).contains(&window), "window out of range");
        assert!((5..=8).contains(&literal), "literal out of range");
        let dict_size = 1 << window;
        let has_dictionary = dictionary.is_some();
        let dict = match dictionary {
            Some(mut d) => {
                if d.len() != dict_size {
                    panic!("Dictionary-window size mismatch.");
                }
                d
            },
            None => {
                let mut d = vec![0u8; dict_size];
                tamp_initialize_dictionary(&mut d);
                d
            }
        };
        let min_pattern_size = tamp_compute_min_pattern_size(window, literal) as usize;
        let max_pattern_size = min_pattern_size + 13;
        let literal_flag = 1u16 << literal;
        let window_buffer = RingBuffer::new(dict);
        let input_buffer = VecDeque::with_capacity(max_pattern_size);
        let mut bit_writer = BitWriter::new(writer, false);
        // Write header
        bit_writer.write((window - 8) as u32, 3, false)?;
        bit_writer.write((literal - 5) as u32, 2, false)?;
        bit_writer.write(if has_dictionary { 1 } else { 0 }, 1, false)?;
        bit_writer.write(0, 1, false)?; // Reserved
        bit_writer.write(0, 1, false)?; // Reserved

        Ok(Compressor {
            bit_writer,
            window_bits: window,
            literal_bits: literal,
            min_pattern_size,
            max_pattern_size,
            literal_flag,
            window_buffer,
            input_buffer,
        })
    }

    fn compress_input_buffer_single(&mut self) -> Result<usize> {
        let target: Vec<u8> = self.input_buffer.iter().copied().collect();
        let mut bytes_written = 0;
        let mut best_match_size = 0;
        let mut best_match_pos = 0;
        for size in (self.min_pattern_size..=target.len()).rev() {
            let match_slice = &target[..size];
            if let Some(idx) = self.window_buffer.index(match_slice, 0) {
                best_match_size = size;
                best_match_pos = idx;
                break; // Longest match found
            }
        }
        let match_slice = &target[..best_match_size];
        if best_match_size >= self.min_pattern_size {
            let match_size = best_match_size;
            let match_pos = best_match_pos;
            bytes_written += self.bit_writer.write_huffman(match_size - self.min_pattern_size)?;
            bytes_written += self.bit_writer.write(match_pos as u32, self.window_bits, true)?;
            self.window_buffer.write_bytes(match_slice);
            for _ in 0..match_size {
                self.input_buffer.pop_front();
            }
        } else {
            let char = self.input_buffer.pop_front().unwrap();
            if (char.checked_shr(self.literal_bits as u32).unwrap_or(0)) != 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "ExcessBitsError"));
            }
            bytes_written += self.bit_writer.write((char as u16 | self.literal_flag) as u32, self.literal_bits + 1, true)?;
            self.window_buffer.write_byte(char);
        }
        Ok(bytes_written)
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let mut bytes_written = 0;
        for &char in data {
            self.input_buffer.push_back(char);
            if self.input_buffer.len() == self.max_pattern_size {
                bytes_written += self.compress_input_buffer_single()?;
            }
        }
        Ok(bytes_written)
    }

    pub fn flush(&mut self, write_token: bool) -> Result<usize> {
        let mut bytes_written = 0;
        while !self.input_buffer.is_empty() {
            bytes_written += self.compress_input_buffer_single()?;
        }
        bytes_written += self.bit_writer.flush(write_token)?;
        Ok(bytes_written)
    }

    pub fn close(mut self) -> Result<usize> {
        let bytes_written = self.flush(false)?;
        // Drop bit_writer, which flushes the underlying writer
        Ok(bytes_written)
    }
}


pub struct TextCompressor<W: Write> {
    compressor: Compressor<W>,
}

impl<W: Write> TextCompressor<W> {
    pub fn new(writer: W, window: u8, literal: u8, dictionary: Option<Vec<u8>>) -> Result<Self> {
        Ok(TextCompressor {
            compressor: Compressor::new(writer, window, literal, dictionary)?,
        })
    }

    pub fn write(&mut self, data: &str) -> Result<usize> {
        self.compressor.write(data.as_bytes())
    }

    pub fn flush(&mut self, write_token: bool) -> Result<usize> {
        self.compressor.flush(write_token)
    }

    pub fn close(self) -> Result<usize> {
        self.compressor.close()
    }
}




pub enum CompressInput<'a> {
    Bytes(&'a [u8]),
    Str(&'a str),
}

pub fn compress(
    data: CompressInput,
    window: u8,
    literal: u8,
    dictionary: Option<Vec<u8>>,
) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    match data {
        CompressInput::Bytes(bytes) => {
            let mut compressor = Compressor::new(&mut output, window, literal, dictionary)?;
            compressor.write(bytes)?;
            compressor.flush(false)?;
        }
        CompressInput::Str(s) => {
            let mut compressor = TextCompressor::new(&mut output, window, literal, dictionary)?;
            compressor.write(s)?;
            compressor.flush(false)?;
        }
    }
    Ok(output)
}