use proc_bitfield::{bits, with_bits};

#[test]
fn ints_rw_int_arrays_basic_functionality() {
    assert_eq!(bits!(0x8765_4321_u32, [u8; 2] @ 8; 16), [0x43_u8, 0x65]);
    assert_eq!(bits!(0x8765_4321_u32, [u8; 2] @ 12; 16), [0x54_u8, 0x76]);

    assert_eq!(
        with_bits!(0x8765_4321_u32, [u8; 2] @ 8; 16 = [0xDC, 0xFE]),
        0x87FE_DC21
    );
    assert_eq!(
        with_bits!(0x8765_4321_u32, [u8; 2] @ 12; 16 = [0xDC, 0xFE]),
        0x8FED_C321
    );

    assert_eq!(bits!(0x21_u8, [u16; 2] @ 0; 4), [1, 0]);
    assert_eq!(
        with_bits!(0x21_u8, [u16; 2] @ 0; 4 = [0xFFFF, 0xFFFF]),
        0x2F
    );
}

#[test]
fn ints_read_int_arrays_signs() {
    assert_eq!(bits!(0xA9A7_6543_u32, [u8; 2] @ 8; 15), [0x65, 0x27]);
    assert_eq!(bits!(0xA9A7_6543_u32 as i32, [u8; 2] @ 8; 15), [0x65, 0x27]);
    assert_eq!(
        bits!(0xA9A7_6543_u32, [i8; 2] @ 8; 15),
        [0x65, 0x27_u8 as i8]
    );
    assert_eq!(
        bits!(0xA9A7_6543_u32 as i32, [i8; 2] @ 8; 15),
        [0x65, 0x27_u8 as i8]
    );

    assert_eq!(bits!(0xA9A7_6543_u32, [u8; 2] @ 8; 14), [0x65, 0x27]);
    assert_eq!(bits!(0xA9A7_6543_u32 as i32, [u8; 2] @ 8; 14), [0x65, 0x27]);
    assert_eq!(
        bits!(0xA9A7_6543_u32, [i8; 2] @ 8; 14),
        [0x65, 0xE7_u8 as i8]
    );
    assert_eq!(
        bits!(0xA9A7_6543_u32 as i32, [i8; 2] @ 8; 14),
        [0x65, 0xE7_u8 as i8]
    );

    assert_eq!(bits!(0xA9A7_6543_u32, [u8; 2] @ 9; 15), [0xB2, 0x53]);
    assert_eq!(bits!(0xA9A7_6543_u32 as i32, [u8; 2] @ 9; 15), [0xB2, 0x53]);
    assert_eq!(
        bits!(0xA9A7_6543_u32, [i8; 2] @ 9; 15),
        [0xB2_u8 as i8, 0xD3_u8 as i8]
    );
    assert_eq!(
        bits!(0xA9A7_6543_u32 as i32, [i8; 2] @ 9; 15),
        [0xB2_u8 as i8, 0xD3_u8 as i8]
    );

    assert_eq!(bits!(0xFF_u8, [u16; 2] @ 0; 4), [0xF, 0]);
    assert_eq!(bits!(-1_i8, [u16; 2] @ 0; 4), [0xF, 0]);
    assert_eq!(bits!(0xFF_u8, [i16; 2] @ 0; 4), [-1, 0]);
    assert_eq!(bits!(-1_i8, [i16; 2] @ 0; 4), [-1, 0]);

    assert_eq!(bits!(0x0F_u8, [u16; 2] @ 0; 4), [0xF, 0]);
    assert_eq!(bits!(0x0F_i8, [u16; 2] @ 0; 4), [0xF, 0]);
    assert_eq!(bits!(0x0F_u8, [i16; 2] @ 0; 4), [-1, 0]);
    assert_eq!(bits!(0x0F_i8, [i16; 2] @ 0; 4), [-1, 0]);

    assert_eq!(bits!(0x0F_u8, [u16; 2] @ 0; 5), [0xF, 0]);
    assert_eq!(bits!(0x0F_i8, [u16; 2] @ 0; 5), [0xF, 0]);
    assert_eq!(bits!(0x0F_u8, [i16; 2] @ 0; 5), [0xF, 0]);
    assert_eq!(bits!(0x0F_i8, [i16; 2] @ 0; 5), [0xF, 0]);
}

#[test]
fn ints_write_int_arrays_signs() {
    assert_eq!(
        with_bits!(0x8765_4321_u32, [u8; 2] @ 8; 15 = [0xFF; 2]),
        0x877F_FF21
    );
    assert_eq!(
        with_bits!(0x8765_4321_u32 as i32, [u8; 2] @ 8; 15 = [0xFF; 2]),
        0x877F_FF21_u32 as i32
    );
    assert_eq!(
        with_bits!(0x8765_4321_u32, [i8; 2] @ 8; 15 = [-1; 2]),
        0x877F_FF21
    );
    assert_eq!(
        with_bits!(0x8765_4321_u32 as i32, [i8; 2] @ 8; 15 = [-1; 2]),
        0x877F_FF21_u32 as i32
    );

    assert_eq!(
        with_bits!(0x87A5_4321_u32, [u8; 2] @ 8; 15 = [0; 2]),
        0x8780_0021
    );
    assert_eq!(
        with_bits!(0x87A5_4321_u32 as i32, [u8; 2] @ 8; 15 = [0; 2]),
        0x8780_0021_u32 as i32
    );
    assert_eq!(
        with_bits!(0x87A5_4321_u32, [i8; 2] @ 8; 15 = [0; 2]),
        0x8780_0021
    );
    assert_eq!(
        with_bits!(0x87A5_4321_u32 as i32, [i8; 2] @ 8; 15 = [0; 2]),
        0x8780_0021_u32 as i32
    );

    assert_eq!(with_bits!(0x21_u8, [u16; 2] @ 0; 4 = [0xFFFF; 2]), 0x2F);
    assert_eq!(with_bits!(0x21_i8, [u16; 2] @ 0; 4 = [0xFFFF; 2]), 0x2F);
    assert_eq!(with_bits!(0x21_u8, [i16; 2] @ 0; 4 = [-1; 2]), 0x2F);
    assert_eq!(with_bits!(0x21_i8, [i16; 2] @ 0; 4 = [-1; 2]), 0x2F);

    assert_eq!(with_bits!(0x21_u8, [u16; 2] @ 0; 4 = [0; 2]), 0x20);
    assert_eq!(with_bits!(0x21_i8, [u16; 2] @ 0; 4 = [0; 2]), 0x20);
    assert_eq!(with_bits!(0x21_u8, [i16; 2] @ 0; 4 = [0; 2]), 0x20);
    assert_eq!(with_bits!(0x21_i8, [i16; 2] @ 0; 4 = [0; 2]), 0x20);
}
