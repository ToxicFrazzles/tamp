// Based on _BitReader from https://raw.githubusercontent.com/BrianPugh/tamp/refs/heads/main/tamp/decompressor.py

use std::io::{Read, Result};
use std::collections::HashMap;

// Huffman lookup table as in decompressor.py
fn huffman_lookup() -> HashMap<u16, u8> {
    let mut map = HashMap::new();
    map.insert(0b0, 0);
    map.insert(0b11, 1);
    map.insert(0b1000, 2);
    map.insert(0b1011, 3);
    map.insert(0b10100, 4);
    map.insert(0b100100, 5);
    map.insert(0b100110, 6);
    map.insert(0b101011, 7);
    map.insert(0b1001011, 8);
    map.insert(0b1010100, 9);
    map.insert(0b10010100, 10);
    map.insert(0b10010101, 11);
    map.insert(0b10101010, 12);
    map.insert(0b100111, 13);
    map.insert(0b10101011, 255); // 255 as FLUSH marker
    map
}

pub struct BitReader<R: Read> {
    reader: R,
    buffer: u32,
    bit_pos: u8,
    close_reader_on_close: bool,
    backup_buffer: Option<u32>,
    backup_bit_pos: Option<u8>,
}

impl<R: Read> BitReader<R> {
    pub fn new(reader: R, close_reader_on_close: bool) -> Self {
        BitReader {
            reader,
            buffer: 0,
            bit_pos: 0,
            close_reader_on_close,
            backup_buffer: None,
            backup_bit_pos: None,
        }
    }

    pub fn read(&mut self, num_bits: u8) -> Result<u32> {
        while self.bit_pos < num_bits {
            let mut byte = [0u8; 1];
            let n = self.reader.read(&mut byte)?;
            if n == 0 {
                return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "EOF"));
            }
            let byte_value = byte[0] as u32;
            self.buffer |= byte_value << (24 - self.bit_pos);
            self.bit_pos += 8;
            if self.backup_buffer.is_some() && self.backup_bit_pos.is_some() {
                let backup_buf = self.backup_buffer.as_mut().unwrap();
                let backup_pos = self.backup_bit_pos.as_mut().unwrap();
                *backup_buf |= byte_value << (24 - *backup_pos);
                *backup_pos += 8;
            }
        }
        let result = self.buffer >> (32 - num_bits);
        let mask = (1 << (32 - num_bits)) - 1;
        self.buffer = (self.buffer & mask) << num_bits;
        self.bit_pos -= num_bits;
        Ok(result)
    }

    pub fn read_huffman(&mut self) -> Result<u8> {
        let lookup = huffman_lookup();
        let mut proposed_code: u16 = 0;
        for _ in 0..8 {
            proposed_code = (proposed_code << 1) | self.read(1)? as u16;
            if let Some(&val) = lookup.get(&proposed_code) {
                return Ok(val);
            }
        }
        Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unable to decode huffman code"))
    }

    pub fn clear(&mut self) {
        self.buffer = 0;
        self.bit_pos = 0;
        self.backup_buffer = None;
        self.backup_bit_pos = None;
    }

    pub fn close(self) {
        // For file, drop is enough
    }

    pub fn len(&self) -> u8 {
        self.bit_pos
    }

    pub fn backup(&mut self) {
        self.backup_buffer = Some(self.buffer);
        self.backup_bit_pos = Some(self.bit_pos);
    }

    pub fn restore(&mut self) {
        if let (Some(buf), Some(pos)) = (self.backup_buffer, self.backup_bit_pos) {
            self.buffer = buf;
            self.bit_pos = pos;
        }
        self.backup_buffer = None;
        self.backup_bit_pos = None;
    }
}
