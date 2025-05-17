/// The UARTDR Register is the data register.
///
/// For words to be transmitted:
///  - if the FIFOs are enabled, data written to this location is pushed onto the transmit FIFO
///  - if the FIFOs are not enabled, data is stored in the transmitter holding register (the bottom word of the transmit FIFO).
/// The write operation initiates transmission from the UART. The data is prefixed with a start bit, appended with the appropriate parity bit (if parity is enabled), and a stop bit. The resultant word is then transmitted.
///
/// For received words:
///  - if the FIFOs are enabled, the data byte and the 4-bit status (break, frame, parity, and overrun) is pushed onto the 12-bit wide receive FIFO
///  - if the FIFOs are not enabled, the data byte and status are stored in the receiving holding register (the bottom word of the receive FIFO).
///
/// The received data byte is read by performing reads from the UARTDR Register along with the corresponding status information. The status information can also be read by a read of the UARTRSR/UARTECR Register
#[bitstuff::stuff(u32)]
#[derive(Default)]
pub struct DataRegister {
    /// This bit is set to 1 if data is received and the receive FIFO is already full.
    /// This is cleared to 0 once there is an empty space in the FIFO and a new character can be written to it.
    #[bitstuff(bit = 11)]
    overrun_error: bool,
    /// This bit is set to 1 if a break condition was detected, indicating that the received data input was held LOW for longer than a full-word transmission time (defined as start, data, parity and stop bits).
    ///
    /// In FIFO mode, this error is associated with the character at the top of the FIFO. When a break occurs, only one 0 character is loaded into the FIFO.
    /// The next character is only enabled after the receive data input goes to a 1 (marking state), and the next valid start bit is received.
    #[bitstuff(bit = 10)]
    break_error: bool,
    ///  When set to 1, it indicates that the parity of the received data character does not match the parity that the EPS and SPS bits in the Line Control Register, UARTLCR_H select.
    /// In FIFO mode, this error is associated with the character at the top of the FIFO.
    #[bitstuff(bit = 9)]
    parity_error: bool,
    /// When set to 1, it indicates that the received character did not have a valid stop bit (a valid stop bit is 1).
    /// In FIFO mode, this error is associated with the character at the top of the FIFO.
    #[bitstuff(bit = 8)]
    framing_error: bool,
    /// Receive (read) data character.
    /// Transmit (write) data character.
    #[bitstuff(bits = 0..8)]
    data: u8,
}

#[bitstuff::stuff]
#[derive(Default)]
pub struct InterruptFIFOLevelSelect {
    /// Receive interrupt FIFO level select. The trigger points for the receive interrupt are as follows:
    ///
    /// - b000 = Receive FIFO becomes ≥ 1/8 full
    /// - b001 = Receive FIFO becomes ≥ 1/4 full
    /// - b010 = Receive FIFO becomes ≥ 1/2 full
    /// - b011 = Receive FIFO becomes ≥ 3/4 full
    /// - b100 = Receive FIFO becomes ≥ 7/8 full
    /// - b101-b111 = reserved.
    #[allow(non_snake_case)]
    #[bitstuff(bits = 3..=5, falliable)]
    receive_interrupt_FIFO_level_select: FIFOLevel, //acutally 3 bits
    /// Transmit interrupt FIFO level select. The trigger points for the transmit interrupt are as follows:
    /// - b000 = Transmit FIFO becomes ≤ 1/8 full
    /// - b001 = Transmit FIFO becomes ≤ 1/4 full
    /// - b010 = Transmit FIFO becomes ≤ 1/2 full
    /// - b011 = Transmit FIFO becomes ≤ 3/4 full
    /// - b100 = Transmit FIFO becomes ≤ 7/8 full
    /// - b101-b111 = reserved.
    #[bitstuff(bits = 0..=2, falliable)]
    #[allow(non_snake_case)]
    transmit_interrupt_FIFO_level_select: FIFOLevel, //acutally 3 bits
}

#[bitstuff::stuff]
#[derive(Default)]
struct Nested {
    #[bitstuff(bits = 0..6)]
    test: InterruptFIFOLevelSelect,
    #[bitstuff(bits = 11..17)]
    test2: InterruptFIFOLevelSelect,
    #[bitstuff(bits = 17..25)]
    test3: FullU8,
    #[bitstuff(bit = 25)]
    test4: EvenOdd,
}

#[bitstuff::stuff(u32, bits = 3)]
#[derive(Debug, Default)]
//check that all unique and maximum fits in amount of bits given
// if nr items = 2^bits then it's a full enum and will implement FromBits, otherwise TryFromBits
pub enum FIFOLevel {
    #[default]
    OneEightFull = 0b000,
    OneFourthFull = 0b001,
    HalfwayFull = 0b010,
    ThreeFourthFull = 0b011,
    SevenEightFull = 0b100,
    // other values reserved
}

//todo: check no attributes on the enum variants
// or allow people to explicitly set the bit width
// could be a way to implement registers which should be read in a specific way. like id registers. Have an enum with only one variant and fixed bit width
#[bitstuff::stuff]
#[derive(Debug, Default)]
//check that all unique and maximum fits in amount of bits given
// if nr items = 2^bits then it's a full enum and will implement FromBits, otherwise TryFromBits
pub enum EvenOdd {
    #[default]
    Even = 0,
    Odd = 1,
}

fn main() {
    println!(
        "{:#?}",
        DataRegister::default()
            .with_data(0xaf_u8)
            .with_framing_error(true)
            .with_parity_error(true)
            .with_overrun_error(false)
            .with_break_error(true)
    );

    // println!("{:#?}", InterruptFIFOLevelSelect(0xFFFFFFFF));
    println!(
        "{:#?}",
        InterruptFIFOLevelSelect::default()
            .with_receive_interrupt_FIFO_level_select(FIFOLevel::HalfwayFull)
            .with_transmit_interrupt_FIFO_level_select(FIFOLevel::OneFourthFull) // .with_test_field(EvenOdd::Odd)
    );

    println!("{:#?}", FullU8::default());

    use ::bitstuff::ToBits;
    let nested = Nested::default()
        .with_test(InterruptFIFOLevelSelect::default())
        .with_test2(InterruptFIFOLevelSelect::default())
        .with_test3(FullU8::X0x01)
        .with_test4(EvenOdd::Even)
        .to_bits();
    // this should be u26
    println!("{:#?}", nested);
}

#[bitstuff::stuff]
#[derive(Debug, Default)]
enum FullU8 {
    X0x00 = 0,
    X0x01 = 1,
    X0x02 = 2,
    X0x03 = 3,
    X0x04 = 4,
    X0x05 = 5,
    X0x06 = 6,
    X0x07 = 7,
    X0x08 = 8,
    X0x09 = 9,
    X0x0a = 10,
    X0x0b = 11,
    X0x0c = 12,
    X0x0d = 13,
    X0x0e = 14,
    X0x0f = 15,
    X0x10 = 16,
    X0x11 = 17,
    X0x12 = 18,
    X0x13 = 19,
    X0x14 = 20,
    X0x15 = 21,
    X0x16 = 22,
    X0x17 = 23,
    X0x18 = 24,
    X0x19 = 25,
    X0x1a = 26,
    X0x1b = 27,
    X0x1c = 28,
    X0x1d = 29,
    X0x1e = 30,
    X0x1f = 31,
    X0x20 = 32,
    X0x21 = 33,
    X0x22 = 34,
    X0x23 = 35,
    X0x24 = 36,
    X0x25 = 37,
    X0x26 = 38,
    X0x27 = 39,
    X0x28 = 40,
    X0x29 = 41,
    X0x2a = 42,
    X0x2b = 43,
    X0x2c = 44,
    X0x2d = 45,
    X0x2e = 46,
    X0x2f = 47,
    X0x30 = 48,
    X0x31 = 49,
    X0x32 = 50,
    X0x33 = 51,
    X0x34 = 52,
    X0x35 = 53,
    X0x36 = 54,
    X0x37 = 55,
    X0x38 = 56,
    X0x39 = 57,
    X0x3a = 58,
    X0x3b = 59,
    X0x3c = 60,
    X0x3d = 61,
    X0x3e = 62,
    X0x3f = 63,
    X0x40 = 64,
    X0x41 = 65,
    X0x42 = 66,
    X0x43 = 67,
    X0x44 = 68,
    X0x45 = 69,
    X0x46 = 70,
    X0x47 = 71,
    X0x48 = 72,
    X0x49 = 73,
    X0x4a = 74,
    X0x4b = 75,
    X0x4c = 76,
    X0x4d = 77,
    X0x4e = 78,
    X0x4f = 79,
    X0x50 = 80,
    X0x51 = 81,
    X0x52 = 82,
    X0x53 = 83,
    X0x54 = 84,
    X0x55 = 85,
    X0x56 = 86,
    X0x57 = 87,
    X0x58 = 88,
    X0x59 = 89,
    X0x5a = 90,
    X0x5b = 91,
    X0x5c = 92,
    X0x5d = 93,
    X0x5e = 94,
    X0x5f = 95,
    X0x60 = 96,
    X0x61 = 97,
    X0x62 = 98,
    X0x63 = 99,
    X0x64 = 100,
    X0x65 = 101,
    X0x66 = 102,
    X0x67 = 103,
    X0x68 = 104,
    X0x69 = 105,
    X0x6a = 106,
    X0x6b = 107,
    X0x6c = 108,
    X0x6d = 109,
    X0x6e = 110,
    X0x6f = 111,
    X0x70 = 112,
    X0x71 = 113,
    X0x72 = 114,
    X0x73 = 115,
    X0x74 = 116,
    X0x75 = 117,
    X0x76 = 118,
    X0x77 = 119,
    X0x78 = 120,
    X0x79 = 121,
    X0x7a = 122,
    X0x7b = 123,
    X0x7c = 124,
    X0x7d = 125,
    X0x7e = 126,
    X0x7f = 127,
    X0x80 = 128,
    X0x81 = 129,
    X0x82 = 130,
    X0x83 = 131,
    X0x84 = 132,
    X0x85 = 133,
    X0x86 = 134,
    X0x87 = 135,
    X0x88 = 136,
    X0x89 = 137,
    X0x8a = 138,
    X0x8b = 139,
    X0x8c = 140,
    X0x8d = 141,
    X0x8e = 142,
    X0x8f = 143,
    X0x90 = 144,
    X0x91 = 145,
    X0x92 = 146,
    X0x93 = 147,
    X0x94 = 148,
    X0x95 = 149,
    X0x96 = 150,
    X0x97 = 151,
    X0x98 = 152,
    X0x99 = 153,
    X0x9a = 154,
    X0x9b = 155,
    X0x9c = 156,
    X0x9d = 157,
    X0x9e = 158,
    X0x9f = 159,
    X0xa0 = 160,
    X0xa1 = 161,
    X0xa2 = 162,
    X0xa3 = 163,
    X0xa4 = 164,
    X0xa5 = 165,
    X0xa6 = 166,
    X0xa7 = 167,
    X0xa8 = 168,
    X0xa9 = 169,
    X0xaa = 170,
    X0xab = 171,
    X0xac = 172,
    X0xad = 173,
    X0xae = 174,
    X0xaf = 175,
    X0xb0 = 176,
    X0xb1 = 177,
    X0xb2 = 178,
    X0xb3 = 179,
    X0xb4 = 180,
    X0xb5 = 181,
    X0xb6 = 182,
    X0xb7 = 183,
    X0xb8 = 184,
    X0xb9 = 185,
    X0xba = 186,
    X0xbb = 187,
    X0xbc = 188,
    X0xbd = 189,
    X0xbe = 190,
    X0xbf = 191,
    X0xc0 = 192,
    X0xc1 = 193,
    X0xc2 = 194,
    X0xc3 = 195,
    X0xc4 = 196,
    X0xc5 = 197,
    X0xc6 = 198,
    X0xc7 = 199,
    X0xc8 = 200,
    X0xc9 = 201,
    X0xca = 202,
    X0xcb = 203,
    X0xcc = 204,
    X0xcd = 205,
    X0xce = 206,
    X0xcf = 207,
    X0xd0 = 208,
    X0xd1 = 209,
    X0xd2 = 210,
    X0xd3 = 211,
    X0xd4 = 212,
    X0xd5 = 213,
    X0xd6 = 214,
    X0xd7 = 215,
    X0xd8 = 216,
    X0xd9 = 217,
    X0xda = 218,
    X0xdb = 219,
    X0xdc = 220,
    X0xdd = 221,
    X0xde = 222,
    X0xdf = 223,
    X0xe0 = 224,
    X0xe1 = 225,
    X0xe2 = 226,
    X0xe3 = 227,
    X0xe4 = 228,
    X0xe5 = 229,
    X0xe6 = 230,
    X0xe7 = 231,
    X0xe8 = 232,
    X0xe9 = 233,
    X0xea = 234,
    X0xeb = 235,
    X0xec = 236,
    X0xed = 237,
    X0xee = 238,
    X0xef = 239,
    X0xf0 = 240,
    X0xf1 = 241,
    X0xf2 = 242,
    X0xf3 = 243,
    X0xf4 = 244,
    X0xf5 = 245,
    X0xf6 = 246,
    X0xf7 = 247,
    X0xf8 = 248,
    X0xf9 = 249,
    X0xfa = 250,
    X0xfb = 251,
    X0xfc = 252,
    X0xfd = 253,
    #[default]
    X0xfe = 254,
    X0xff = 255,
}
