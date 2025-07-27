
use std::io::{Write, Result};

pub const HUFFMAN_CODES: [u16; 14] = [
    0x00, 0x03, 0x08, 0x0b, 0x14, 0x24, 0x26, 0x2b, 0x4b, 0x54, 0x94, 0x95, 0xaa, 0x27
];
pub const HUFFMAN_BITS: [u8; 14] = [
    2, 3, 5, 5, 6, 7, 7, 7, 8, 8, 9, 9, 9, 7
];
const FLUSH_CODE: u16 = 0xAB;

pub struct BitWriter<W: Write> {
    writer: W,
    buffer: u32, // 24 bits used
    bit_pos: u8, // number of bits in buffer
    close_writer_on_close: bool,
}

impl<W: Write> BitWriter<W> {
    /// Get the current buffer (for debugging)
    pub fn get_buffer(&self) -> u32 {
        self.buffer
    }

    /// Get the current bit position (for debugging)
    pub fn get_bit_pos(&self) -> u8 {
        self.bit_pos
    }
    pub fn new(writer: W, close_writer_on_close: bool) -> Self {
        BitWriter {
            writer,
            buffer: 0,
            bit_pos: 0,
            close_writer_on_close,
        }
    }

    /// Write up to 32 bits to the stream, most significant bit first.
    pub fn write(&mut self, mut bits: u32, num_bits: u8, flush: bool) -> Result<usize> {
        bits &= (1u32 << num_bits) - 1;
        self.bit_pos += num_bits;
        self.buffer |= bits << (32 - self.bit_pos);
        let mut bytes_written = 0;
        if flush {
            while self.bit_pos >= 8 {
                let byte = (self.buffer >> 24) as u8;
                self.writer.write_all(&[byte])?;
                self.buffer = (self.buffer & 0xFFFFFF) << 8;
                self.bit_pos -= 8;
                bytes_written += 1;
            }
        }
        Ok(bytes_written)
    }

    /// Write a pattern size using the static Huffman table.
    pub fn write_huffman(&mut self, pattern_size: usize) -> Result<usize> {
        if pattern_size >= HUFFMAN_CODES.len() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "pattern_size out of range"));
        }
        let code = HUFFMAN_CODES[pattern_size] as u32;
        let bits = HUFFMAN_BITS[pattern_size];
        self.write(code, bits, true)
    }

    /// Flushes the buffer, optionally writing a FLUSH token (0xAB, 9 bits)
    pub fn flush(&mut self, write_token: bool) -> Result<usize> {
        let mut bytes_written = 0;
        if self.bit_pos > 0 && write_token {
            bytes_written += self.write(FLUSH_CODE as u32, 9, true)?;
        }
        while self.bit_pos > 0 {
            let byte = ((self.buffer >> 24) & 0xFF) as u8;
            self.writer.write_all(&[byte])?;
            self.buffer = (self.buffer & 0xFFFFFF) << 8;
            self.bit_pos = self.bit_pos.saturating_sub(8);
            bytes_written += 1;
        }
        self.buffer = 0;
        self.bit_pos = 0;
        self.writer.flush()?;
        Ok(bytes_written)
    }

    /// Close the writer, flushing all buffers. Returns number of bytes written on close.
    pub fn close(mut self) -> Result<usize> {
        let bytes_written = self.flush(false)?;
        // If the writer needs to be closed, drop it here (for File, flush is enough)
        Ok(bytes_written)
    }
}
