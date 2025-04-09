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

#[bitstuff::stuff(u32)]
#[derive(Default)]
pub struct InterruptFIFOLevelSelect {
    #[bitstuff(bit = 31)]
    test_field: EvenOdd,
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

    println!("{:#?}", InterruptFIFOLevelSelect(0xFFFFFFFF));
    println!(
        "{:#?}",
        InterruptFIFOLevelSelect::default()
            .with_receive_interrupt_FIFO_level_select(FIFOLevel::HalfwayFull)
            .with_transmit_interrupt_FIFO_level_select(FIFOLevel::OneFourthFull)
            .with_test_field(EvenOdd::Odd)
    );
}
