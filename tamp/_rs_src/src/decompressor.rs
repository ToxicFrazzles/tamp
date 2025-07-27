// Based on https://raw.githubusercontent.com/BrianPugh/tamp/refs/heads/main/tamp/decompressor.py
use std::io::{Read, Result};
use crate::bit_reader::BitReader;
use crate::ring_buffer::RingBuffer;
use crate::common::{tamp_initialize_dictionary, tamp_compute_min_pattern_size};

pub struct Decompressor<R: Read> {
    bit_reader: BitReader<R>,
    window_bits: u8,
    literal_bits: u8,
    min_pattern_size: usize,
    window_buffer: RingBuffer,
    overflow: Vec<u8>,
}

impl<R: Read> Decompressor<R> {
    pub fn new(reader: R, mut dictionary: Option<Vec<u8>>) -> Result<Self> {
        let mut bit_reader = BitReader::new(reader, false);
        // Read header
        let window_bits = bit_reader.read(3)? as u8 + 8;
        let literal_bits = bit_reader.read(2)? as u8 + 5;
        let uses_custom_dictionary = bit_reader.read(1)? != 0;
        let reserved = bit_reader.read(1)?;
        let more_header_bytes = bit_reader.read(1)?;
        if reserved != 0 || more_header_bytes != 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unsupported header flags"));
        }
        if uses_custom_dictionary && dictionary.is_none() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Custom dictionary required but not provided"));
        }
        let dict_size = 1 << window_bits;
        let dict = match dictionary {
            Some(d) => {
                if d.len() != dict_size {
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Dictionary-window size mismatch."));
                }
                d
            },
            None => {
                let mut d = vec![0u8; dict_size];
                tamp_initialize_dictionary(&mut d);
                d
            }
        };
        let min_pattern_size = tamp_compute_min_pattern_size(window_bits, literal_bits) as usize;
        let window_buffer = RingBuffer::new(dict);
        Ok(Decompressor {
            bit_reader,
            window_bits,
            literal_bits,
            min_pattern_size,
            window_buffer,
            overflow: Vec::new(),
        })
    }

    pub fn read_into(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut written = 0;
        if self.overflow.len() > buf.len() {
            buf.copy_from_slice(&self.overflow[..buf.len()]);
            let rest = self.overflow.split_off(buf.len());
            self.overflow = rest;
            return Ok(buf.len());
        } else if !self.overflow.is_empty() {
            let n = self.overflow.len();
            buf[..n].copy_from_slice(&self.overflow);
            written += n;
            self.overflow.clear();
        }
        while written < buf.len() {
            self.bit_reader.backup();
            let is_literal = match self.bit_reader.read(1) {
                Ok(b) => b != 0,
                Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e),
            };
            if is_literal {
                let c = match self.bit_reader.read(self.literal_bits) {
                    Ok(val) => val as u8,
                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };
                self.window_buffer.write_byte(c);
                buf[written] = c;
                written += 1;
            } else {
                let match_size = match self.bit_reader.read_huffman() {
                    Ok(val) => val,
                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };
                if match_size == 255 {
                    self.bit_reader.clear();
                    continue;
                }
                let match_size = match_size as usize + self.min_pattern_size;
                let index = match self.bit_reader.read(self.window_bits) {
                    Ok(val) => val as usize,
                    Err(ref e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };
                let mut string = Vec::with_capacity(match_size);
                for i in 0..match_size {
                    let idx = (index + i) % self.window_buffer.size;
                    string.push(self.window_buffer.buffer[idx]);
                }
                self.window_buffer.write_bytes(&string);
                let to_buf = std::cmp::min(buf.len() - written, match_size);
                buf[written..written + to_buf].copy_from_slice(&string[..to_buf]);
                written += to_buf;
                if to_buf < match_size {
                    self.overflow = string[to_buf..].to_vec();
                    break;
                }
            }
        }
        Ok(written)
    }
}


pub struct TextDecompressor<R: Read> {
    decompressor: Decompressor<R>,
}

impl<R: Read> TextDecompressor<R> {
    pub fn new(reader: R, dictionary: Option<Vec<u8>>) -> Result<Self> {
        Ok(TextDecompressor {
            decompressor: Decompressor::new(reader, dictionary)?,
        })
    }

    pub fn read_to_string(&mut self, size: Option<usize>) -> Result<String> {
        let mut buf = Vec::new();
        match size {
            Some(n) => {
                buf.resize(n, 0);
                let read = self.decompressor.read_into(&mut buf)?;
                buf.truncate(read);
            }
            None => {
                let mut chunk = vec![0u8; 4096];
                loop {
                    let read = self.decompressor.read_into(&mut chunk)?;
                    if read == 0 {
                        break;
                    }
                    buf.extend_from_slice(&chunk[..read]);
                }
            }
        }
        Ok(String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?)
    }
}


pub fn decompress(data: &[u8], dictionary: Option<Vec<u8>>) -> Result<Vec<u8>> {
    use std::io::Cursor;
    let mut decompressor = Decompressor::new(Cursor::new(data), dictionary)?;
    let mut buf = Vec::new();
    let mut chunk = vec![0u8; 4096];
    loop {
        let read = decompressor.read_into(&mut chunk)?;
        if read == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..read]);
    }
    Ok(buf)
}