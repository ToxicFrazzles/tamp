pub const COMMON_CHARACTERS: [u8; 16] = [0x20, 0x00, 0x30, 0x65, 0x69, 0x3e, 0x74, 0x6f,
                                        0x3c, 0x61, 0x6e, 0x73, 0xa,  0x72, 0x2f, 0x2e];

#[inline]
fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

pub fn tamp_initialize_dictionary(buffer: &mut [u8]) {
    let mut seed: u32 = 3758097560;
    let mut randbuf: u32 = 0;
    for (i, slot) in buffer.iter_mut().enumerate() {
        if (i & 0x7) == 0 {
            randbuf = xorshift32(&mut seed);
        }
        *slot = COMMON_CHARACTERS[(randbuf & 0x0F) as usize];
        randbuf >>= 4;
    }
}

pub fn tamp_compute_min_pattern_size(window: u8, literal: u8) -> i8 {
    2 + if window > (10 + ((literal - 5) << 1)) { 1 } else { 0 }
}


