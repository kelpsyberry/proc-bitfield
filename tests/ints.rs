use proc_bitfield::{bits, with_bits};

#[test]
fn ints_rw_ints_basic_functionality() {
    assert_eq!(bits!(0x1234_u16, u8 @ 5; 4), 1);
    assert_eq!(with_bits!(0x1234_u16, u8 @ 5; 4 = 0xF), 0x13F4);
}

#[test]
fn ints_read_ints_signs() {
    assert_eq!(bits!(0x0FFF_u16, i8 @ 0..4), -1);
    assert_eq!(bits!(0x0FFF_i16, i8 @ 0..4), -1);
    assert_eq!(bits!(0x0FFF_u16, u8 @ 0..4), 0xF);
    assert_eq!(bits!(0x0FFF_i16, u8 @ 0..4), 0xF);

    assert_eq!(bits!(0xFFFF_u16, i8 @ 0..4), -1);
    assert_eq!(bits!(-1_i16, i8 @ 0..4), -1);
    assert_eq!(bits!(0xFFFF_u16, u8 @ 0..4), 0xF);
    assert_eq!(bits!(-1_i16, u8 @ 0..4), 0xF);

    assert_eq!(bits!(0x0F_u8, i16 @ 0..4), -1);
    assert_eq!(bits!(0x0F_i8, i16 @ 0..4), -1);
    assert_eq!(bits!(0x0F_u8, u16 @ 0..4), 0xF);
    assert_eq!(bits!(0x0F_i8, u16 @ 0..4), 0xF);

    assert_eq!(bits!(0xFF_u8, i16 @ 0..4), -1);
    assert_eq!(bits!(-1_i8, i16 @ 0..4), -1);
    assert_eq!(bits!(0xFF_u8, u16 @ 0..4), 0xF);
    assert_eq!(bits!(-1_i8, u16 @ 0..4), 0xF);
}

#[test]
fn ints_write_ints_signs() {
    assert_eq!(with_bits!(0_u16, i8 @ 0..4 = -1), 0xF);
    assert_eq!(with_bits!(0_i16, i8 @ 0..4 = -1), 0xF);
    assert_eq!(with_bits!(0_u16, u8 @ 0..4 = 0xFF), 0xF);
    assert_eq!(with_bits!(0_i16, u8 @ 0..4 = 0xFF), 0xF);

    assert_eq!(with_bits!(0_u8, i16 @ 0..4 = -1), 0xF);
    assert_eq!(with_bits!(0_i8, i16 @ 0..4 = -1), 0xF);
    assert_eq!(with_bits!(0_u8, u16 @ 0..4 = 0xFFFF), 0xF);
    assert_eq!(with_bits!(0_i8, u16 @ 0..4 = 0xFFFF), 0xF);
}
