pub struct RingBuffer {
    pub buffer: Vec<u8>,
    pub size: usize,
    pos: usize,
}

impl RingBuffer {
    pub fn new(buffer: Vec<u8>) -> Self {
        let size = buffer.len();
        RingBuffer {
            buffer,
            size,
            pos: 0,
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.buffer[self.pos] = byte;
        self.pos = (self.pos + 1) % self.size;
    }

    pub fn write_bytes(&mut self, data: &[u8]) {
        for &byte in data {
            self.write_byte(byte);
        }
    }

    pub fn index(&self, pattern: &[u8], start: usize) -> Option<usize> {
        // Search for pattern in buffer, starting at 'start'.
        if pattern.is_empty() || pattern.len() > self.size {
            return None;
        }
        let mut i = start;
        let mut checked = 0;
        while checked <= self.size - pattern.len() {
            let mut found = true;
            for j in 0..pattern.len() {
                let idx = (i + j) % self.size;
                if self.buffer[idx] != pattern[j] {
                    found = false;
                    break;
                }
            }
            if found {
                return Some(i);
            }
            i = (i + 1) % self.size;
            checked += 1;
        }
        None
    }
}
