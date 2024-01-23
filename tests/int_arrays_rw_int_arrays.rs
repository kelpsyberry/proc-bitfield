use proc_bitfield::{bits, with_bits};

#[test]
fn int_arrays_rw_int_arrays_basic_functionality() {
    assert_eq!(bits!([0x4321_u16, 0x8765], [u8; 2] @ 0; 16), [0x21, 0x43]);
    assert_eq!(bits!([0x4321_u16, 0x8765], [u8; 2] @ 8; 16), [0x43, 0x65]);
    assert_eq!(bits!([0x4321_u16, 0x8765], [u8; 2] @ 12; 16), [0x54, 0x76]);

    assert_eq!(with_bits!([0x4321_u16, 0x8765], [u8; 2] @ 0; 16 = [0xDC, 0xFE]), [0xFEDC, 0x8765]);
    assert_eq!(with_bits!([0x4321_u16, 0x8765], [u8; 2] @ 8; 16 = [0xDC, 0xFE]), [0xDC21, 0x87FE]);
    assert_eq!(with_bits!([0x4321_u16, 0x8765], [u8; 2] @ 12; 16 = [0xDC, 0xFE]), [0xC321, 0x8FED]);

    assert_eq!(bits!([0x21_u8, 0x43], [u16; 2] @ 4; 8), [0x32, 0]);
    assert_eq!(with_bits!([0x21_u8, 0x43], [u16; 2] @ 4; 8 = [0xFFFF, 0xFFFF]), [0xF1, 0x4F]);
}
