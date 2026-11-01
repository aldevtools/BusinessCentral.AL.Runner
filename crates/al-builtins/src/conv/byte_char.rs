//! The character a Byte converts to and the byte a Char converts back to. BC maps every byte through one fixed table (code page
//! 437 read as Windows-1252): 139 is 239 (`ï`), 128 is 199, 177 the euro sign 8364, 255 is 160, and below 128 only 15, 20
//! and 21 move (to 164, 182, 167). The table is a permutation, and a Char of the table goes back to its byte (Char 8364 is
//! Byte 177). Pinned whole by types/lt-byte-char-high/ByteCharHigh.al:111-180 (ByteChar_Codes_*, every byte and its way
//! back) and :182-256 (ByteChar_139_Details, ByteChar_CharAbove255_ToByte), writer Y57.

/// The character code of each byte.
const BYTE_TO_CHAR: [u16; 256] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 164,
    16, 17, 18, 19, 182, 167, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
    48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
    64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
    96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111,
    112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122, 123, 124, 125, 126, 127,
    199, 252, 233, 226, 228, 224, 229, 231, 234, 235, 232, 239, 238, 236, 196, 197,
    201, 230, 198, 244, 246, 242, 251, 249, 255, 214, 220, 162, 163, 165, 15, 402,
    225, 237, 243, 250, 241, 209, 170, 186, 191, 20, 172, 189, 188, 161, 171, 187,
    21, 8364, 129, 8218, 8222, 8230, 8224, 8225, 710, 8240, 352, 8249, 338, 141, 381, 143,
    144, 8216, 8217, 8220, 8221, 8226, 8211, 8212, 732, 8482, 353, 8250, 339, 157, 382, 376,
    168, 169, 173, 174, 175, 179, 180, 184, 185, 190, 192, 193, 194, 166, 195, 200,
    202, 223, 203, 204, 205, 206, 181, 207, 208, 210, 211, 212, 213, 215, 216, 217,
    218, 177, 219, 221, 222, 227, 247, 240, 176, 245, 183, 248, 253, 178, 254, 160,
];

/// The character a Byte converts to.
pub fn byte_to_char(b: u8) -> u16 {
    BYTE_TO_CHAR[usize::from(b)]
}

/// The byte a Char of the table converts to; `None` for a character no byte gives.
pub fn char_to_byte(c: u16) -> Option<u8> {
    BYTE_TO_CHAR.iter().position(|x| *x == c).map(|i| i as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_is_a_permutation_that_maps_back() {
        for b in 0..=255u8 {
            assert_eq!(char_to_byte(byte_to_char(b)), Some(b));
        }
        assert_eq!(byte_to_char(139), 239);
        assert_eq!(char_to_byte(8364), Some(177));
        assert_eq!(char_to_byte(256), None);
    }
}
