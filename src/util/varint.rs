const MAX_VARINT_LEN_U64: usize = 10;

/// Encodes a u64 into given vec and returns the number of bytes written.
/// Using little endian style.
/// https://developers.google.com/protocol-buffers/docs/encoding#varints
pub fn write_u64(data: &mut [u8], mut n: u64) -> usize {
    let mut i = 0;
    // n>128
    while n > 0b1000_0000 {
        // 取低7位
        data[i] = (n as u8) | 0b1000_0000;
        n >>= 7;

        i += 1;
    }
    data[i] = n as u8;
    return i + 1;
}

pub fn read_u64(data: &[u8]) -> (u64, isize) {
    let mut n: u64 = 0;
    let mut shift: u32 = 0;

    for (i, &b) in data.iter().enumerate() {
        // 128
        if b < 0b1000_0000 {
            return match (u64::from(b)).checked_shl(shift) {
                None => (0, -(i as isize + 1)),
                Some(b) => (n | b, (i + 1) as isize),
            };
        }
        // 127
        match ((u64::from(b)) & 0b0111_1111).checked_shl(shift) {
            None => return (0, -(i as isize)),
            Some(b) => n |= b,
        }
        shift += 7;
    }
    (0, 0)
}

#[cfg(test)]
mod tests {
    use super::{read_u64, write_u64, MAX_VARINT_LEN_U64};

    #[test]
    fn test_write_u64() {
        // (input u64 , expected bytes)
        let tests = vec![
            (0u64, vec![0]),
            (100u64, vec![0b0110_0100]),
            (129u64, vec![0b1000_0001, 0b1]),
            (258u64, vec![0b1000_0010, 0b10]),
            (
                58962304u64,
                vec![0b1000_0000, 0b1110_0011, 0b1000_1110, 0b1_1100],
            ),
        ];

        for (input, results) in tests {
            let mut bytes = Vec::with_capacity(MAX_VARINT_LEN_U64);
            for _ in 0..results.len() {
                bytes.push(0);
            }
            let written = write_u64(&mut bytes, input);
            assert_eq!(written,results.len());
            for (i, b) in bytes.iter().enumerate() {
                assert_eq!(results[i], *b);
            }

        }
    }

    #[test]
    fn test_read_u64() {
        #[rustfmt::skip]
            let mut test_data = vec![
            0,
            0b110_0100,
            0b1000_0001, 0b1,
            0b1000_0010, 0b10,
            0b1000_0000, 0b1110_0011, 0b1000_1110, 0b1_1100,
            0b1100_1110, 0b1000_0001, 0b1011_0101, 0b1101_1001, 0b1111_0110, 0b1010_1100, 0b1100_1110, 0b1000_0001, 0b1011_0101, 0b1101_1001, 0b1111_0110, 0b1010_1100,
        ];
        let expects = vec![
            (0u64, 1),
            (100u64, 1),
            (129u64, 2),
            (258u64, 2),
            (58962304u64, 4),
            (0u64, -10), // indicates that a overflow occurs
        ];
        let mut idx = 0;
        while !test_data.is_empty() {
            let (i, size) = read_u64(&test_data);
            if size < 0 {
                // remove all remaining bytes when overflow occurs
                test_data.drain(0..test_data.len());
            } else {
                test_data.drain(0..size as usize);
            }
            let (expect_uint, expect_size) = expects[idx];
            assert_eq!(i, expect_uint);
            assert_eq!(size, expect_size as isize);
            idx += 1;
        }
    }
}