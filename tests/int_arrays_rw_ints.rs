use proc_bitfield::{bits, with_bits};

#[test]
fn int_arrays_rw_ints_basic_functionality() {
    assert_eq!(bits!([0x21_u8, 0x43], u8 @ 4; 8), 0x32);
    assert_eq!(with_bits!([0x21_u8, 0x43], u8 @ 4; 8 = 0xFE), [0xE1, 0x4F]);
}

#[test]
fn int_arrays_read_ints_signs_basic() {
    assert_eq!(bits!([0x21_i8, 0x43], u8 @ 0; 8), 0x21);

    assert_eq!(bits!([0x0FFF_u16, 0], i8 @ 0..4), -1);
    assert_eq!(bits!([0x0FFF_i16, 0], i8 @ 0..4), -1);
    assert_eq!(bits!([0x0FFF_u16, 0], u8 @ 0..4), 0xF);
    assert_eq!(bits!([0x0FFF_i16, 0], u8 @ 0..4), 0xF);

    assert_eq!(bits!([0xFFFF_u16, 0], i8 @ 0..4), -1);
    assert_eq!(bits!([-1_i16, 0], i8 @ 0..4), -1);
    assert_eq!(bits!([0xFFFF_u16, 0], u8 @ 0..4), 0xF);
    assert_eq!(bits!([-1_i16, 0], u8 @ 0..4), 0xF);

    assert_eq!(bits!([0x0F_u8, 0], i16 @ 0..4), -1);
    assert_eq!(bits!([0x0F_i8, 0], i16 @ 0..4), -1);
    assert_eq!(bits!([0x0F_u8, 0], u16 @ 0..4), 0xF);
    assert_eq!(bits!([0x0F_i8, 0], u16 @ 0..4), 0xF);

    assert_eq!(bits!([0xFF_u8, 0], i16 @ 0..4), -1);
    assert_eq!(bits!([-1_i8, 0], i16 @ 0..4), -1);
    assert_eq!(bits!([0xFF_u8, 0], u16 @ 0..4), 0xF);
    assert_eq!(bits!([-1_i8, 0], u16 @ 0..4), 0xF);
}

#[test]
fn int_arrays_read_ints_signs_crossing_boundaries() {
    assert_eq!(bits!([0x21_i8, 0x43], u8 @ 4; 8), 0x32);

    assert_eq!(bits!([0xFFFF_u16, 0x0FFF], i8 @ 14..18), -1);
    assert_eq!(bits!([-1_i16, 0x0FFF], i8 @ 14..18), -1);
    assert_eq!(bits!([0xFFFF_u16, 0x0FFF], u8 @ 14..18), 0xF);
    assert_eq!(bits!([-1_i16, 0x0FFF], u8 @ 14..18), 0xF);

    assert_eq!(bits!([0xFFFF_u16; 2], i8 @ 14..18), -1);
    assert_eq!(bits!([-1_i16; 2], i8 @ 14..18), -1);
    assert_eq!(bits!([0xFFFF_u16; 2], u8 @ 14..18), 0xF);
    assert_eq!(bits!([-1_i16; 2], u8 @ 14..18), 0xF);

    assert_eq!(bits!([0xFF_u8, 0x0F], i16 @ 6..10), -1);
    assert_eq!(bits!([-1_i8, 0x0F], i16 @ 6..10), -1);
    assert_eq!(bits!([0xFF_u8, 0x0F], u16 @ 6..10), 0xF);
    assert_eq!(bits!([-1_i8, 0x0F], u16 @ 6..10), 0xF);

    assert_eq!(bits!([0xFF_u8; 2], i16 @ 6..10), -1);
    assert_eq!(bits!([-1_i8; 2], i16 @ 6..10), -1);
    assert_eq!(bits!([0xFF_u8; 2], u16 @ 6..10), 0xF);
    assert_eq!(bits!([-1_i8; 2], u16 @ 6..10), 0xF);
}

#[test]
fn int_arrays_write_ints_signs_basic() {
    assert_eq!(
        with_bits!([0x21_i8, 0x43], u8 @ 0; 8 = 0xFE),
        [0xFE_u8 as i8, 0x43]
    );

    assert_eq!(with_bits!([0_u16; 2], i8 @ 0..4 = -1), [0xF, 0]);
    assert_eq!(with_bits!([0_i16; 2], i8 @ 0..4 = -1), [0xF, 0]);
    assert_eq!(with_bits!([0_u16; 2], u8 @ 0..4 = 0xFF), [0xF, 0]);
    assert_eq!(with_bits!([0_i16; 2], u8 @ 0..4 = 0xFF), [0xF, 0]);

    assert_eq!(with_bits!([0_u8; 2], i16 @ 0..4 = -1), [0xF, 0]);
    assert_eq!(with_bits!([0_i8; 2], i16 @ 0..4 = -1), [0xF, 0]);
    assert_eq!(with_bits!([0_u8; 2], u16 @ 0..4 = 0xFFFF), [0xF, 0]);
    assert_eq!(with_bits!([0_i8; 2], u16 @ 0..4 = 0xFFFF), [0xF, 0]);
}

#[test]
fn int_arrays_write_ints_signs_crossing_boundaries() {
    assert_eq!(
        with_bits!([0x21_i8, 0x43], u8 @ 4; 8 = 0xFE),
        [0xE1_u8 as i8, 0x4F]
    );

    assert_eq!(with_bits!([0_u16; 2], i8 @ 14..18 = -1), [0xC000, 3]);
    assert_eq!(
        with_bits!([0_i16; 2], i8 @ 14..18 = -1),
        [0xC000_u16 as i16, 3]
    );
    assert_eq!(with_bits!([0_u16; 2], u8 @ 14..18 = 0xFF), [0xC000, 3]);
    assert_eq!(
        with_bits!([0_i16; 2], u8 @ 14..18 = 0xFF),
        [0xC000_u16 as i16, 3]
    );

    assert_eq!(with_bits!([0_u8; 2], i16 @ 6..10 = -1), [0xC0, 3]);
    assert_eq!(with_bits!([0_i8; 2], i16 @ 6..10 = -1), [0xC0_u8 as i8, 3]);
    assert_eq!(with_bits!([0_u8; 2], u16 @ 6..10 = 0xFFFF), [0xC0, 3]);
    assert_eq!(
        with_bits!([0_i8; 2], u16 @ 6..10 = 0xFFFF),
        [0xC0_u8 as i8, 3]
    );
}
