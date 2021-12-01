//! Bitcoin genesis hashes.

#[rustfmt::skip]
/// Bitcoin mainnet genesis hash.
pub const MAINNET: &[u8; 32] = &[
    0x6f, 0xe2, 0x8c, 0x0a, 0xb6, 0xf1, 0xb3, 0x72,
    0xc1, 0xa6, 0xa2, 0x46, 0xae, 0x63, 0xf7, 0x4f,
    0x93, 0x1e, 0x83, 0x65, 0xe1, 0x5a, 0x08, 0x9c,
    0x68, 0xd6, 0x19, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[rustfmt::skip]
/// Bitcoin testnet genesis hash.
pub const TESTNET: &[u8; 32] = &[
    0xca, 0x4f, 0xf6, 0x53, 0xcf, 0x52, 0xdc, 0xe5,
    0x91, 0x52, 0x86, 0x23, 0x8c, 0xc5, 0x34, 0xb2,
    0x21, 0xfb, 0xa8, 0x62, 0xfe, 0x7f, 0x02, 0x36,
    0xf4, 0xca, 0xbe, 0x12, 0xcc, 0xad, 0x22, 0xaa,
];

#[rustfmt::skip]
/// Bitcoin regtest genesis hash.
pub const REGTEST: &[u8; 32] = &[
    0x06, 0x22, 0x6e, 0x46, 0x11, 0x1a, 0x0b, 0x59,
    0xca, 0xaf, 0x12, 0x60, 0x43, 0xeb, 0x5b, 0xbf,
    0x28, 0xc3, 0x4f, 0x3a, 0x5e, 0x33, 0x2a, 0x1f,
    0xc7, 0xb2, 0xb7, 0x3c, 0xf1, 0x88, 0x91, 0x0f,
];
