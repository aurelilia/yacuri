use crate::{AssemblerBuffer, Label};
use alloc::vec::Vec;
use core::{convert::TryInto, mem};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Register(u8);

impl Register {
    pub fn new(value: u8) -> Register {
        assert!(value < 16);
        Register(value)
    }

    fn low_bits(self) -> u8 {
        self.0 & 0b111
    }

    fn value(self) -> u8 {
        self.0
    }

    fn needs_rex(self) -> bool {
        self.0 > 7
    }
}

pub const RAX: Register = Register(0);
pub const RCX: Register = Register(1);
pub const RDX: Register = Register(2);
pub const RBX: Register = Register(3);
pub const RSP: Register = Register(4);
pub const RBP: Register = Register(5);
pub const RSI: Register = Register(6);
pub const RDI: Register = Register(7);

pub const R8: Register = Register(8);
pub const R9: Register = Register(9);
pub const R10: Register = Register(10);
pub const R11: Register = Register(11);
pub const R12: Register = Register(12);
pub const R13: Register = Register(13);
pub const R14: Register = Register(14);
pub const R15: Register = Register(15);

pub const XMM0: XmmRegister = XmmRegister(0);
pub const XMM1: XmmRegister = XmmRegister(1);
pub const XMM2: XmmRegister = XmmRegister(2);
pub const XMM3: XmmRegister = XmmRegister(3);
pub const XMM4: XmmRegister = XmmRegister(4);
pub const XMM5: XmmRegister = XmmRegister(5);
pub const XMM6: XmmRegister = XmmRegister(6);
pub const XMM7: XmmRegister = XmmRegister(7);

pub const XMM8: XmmRegister = XmmRegister(8);
pub const XMM9: XmmRegister = XmmRegister(9);
pub const XMM10: XmmRegister = XmmRegister(10);
pub const XMM11: XmmRegister = XmmRegister(11);
pub const XMM12: XmmRegister = XmmRegister(12);
pub const XMM13: XmmRegister = XmmRegister(13);
pub const XMM14: XmmRegister = XmmRegister(14);
pub const XMM15: XmmRegister = XmmRegister(15);

struct ForwardJump {
    offset: u32,
    label: Label,
    distance: JumpDistance,
}

pub enum JumpDistance {
    Near,
    Far,
}

pub struct AssemblerX64 {
    unresolved_jumps: Vec<ForwardJump>,
    buffer: AssemblerBuffer,
}

impl AssemblerX64 {
    pub fn new() -> AssemblerX64 {
        AssemblerX64 {
            unresolved_jumps: Vec::new(),
            buffer: AssemblerBuffer::new(),
        }
    }

    pub fn create_label(&mut self) -> Label {
        self.buffer.create_label()
    }

    pub fn create_and_bind_label(&mut self) -> Label {
        self.buffer.create_and_bind_label()
    }

    pub fn bind_label(&mut self, lbl: Label) {
        self.buffer.bind_label(lbl);
    }

    fn offset(&self, lbl: Label) -> Option<u32> {
        self.buffer.offset(lbl)
    }

    pub fn finalize(mut self) -> Vec<u8> {
        self.resolve_jumps();
        self.buffer.code
    }

    pub fn position(&self) -> usize {
        self.buffer.position()
    }

    pub fn set_position(&mut self, pos: usize) {
        self.buffer.set_position(pos);
    }

    pub fn set_position_end(&mut self) {
        self.buffer.set_position_end();
    }

    pub fn emit_u8(&mut self, value: u8) {
        self.buffer.emit_u8(value);
    }

    pub fn emit_u32(&mut self, value: u32) {
        self.buffer.emit_u32(value);
    }

    pub fn emit_u64(&mut self, value: u64) {
        self.buffer.emit_u64(value);
    }
}

impl AssemblerX64 {
    fn resolve_jumps(&mut self) {
        let unresolved_jumps = mem::replace(&mut self.unresolved_jumps, Vec::new());

        let old_position = self.position();

        for jump in unresolved_jumps {
            let lbl_offset = self.offset(jump.label).expect("unbound label");
            self.set_position(jump.offset as usize);

            match jump.distance {
                JumpDistance::Near => {
                    let distance: i32 = lbl_offset as i32 - (jump.offset as i32 + 1);
                    assert!(-128 <= distance && distance < 128);
                    self.emit_u8(distance as u8);
                }

                JumpDistance::Far => {
                    let distance: i32 = lbl_offset as i32 - (jump.offset as i32 + 4);
                    self.emit_u32(distance as u32);
                }
            }
        }

        self.set_position(old_position);
    }

    pub fn pushq_r(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0x50 + reg.low_bits());
    }

    pub fn popq_r(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0x58 + reg.low_bits());
    }

    pub fn int3(&mut self) {
        self.emit_u8(0xCC);
    }

    pub fn retq(&mut self) {
        self.emit_u8(0xC3);
    }

    pub fn nop(&mut self) {
        self.emit_u8(0x90);
    }

    pub fn setcc_r(&mut self, condition: Condition, dest: Register) {
        if dest.needs_rex() || dest.low_bits() > 3 {
            self.emit_rex(false, false, false, dest.needs_rex());
        }

        self.emit_u8(0x0F);
        self.emit_u8((0x90 + condition.int()) as u8);
        self.emit_modrm_opcode(0, dest);
    }

    pub fn cmovl(&mut self, condition: Condition, dest: Register, src: Register) {
        self.emit_rex32_optional(dest, src);
        self.emit_u8(0x0F);
        self.emit_u8((0x40 + condition.int()) as u8);
        self.emit_modrm_registers(dest, src);
    }

    pub fn cmovq(&mut self, condition: Condition, dest: Register, src: Register) {
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0F);
        self.emit_u8((0x40 + condition.int()) as u8);
        self.emit_modrm_registers(dest, src);
    }

    pub fn lea(&mut self, dest: Register, src: Address) {
        self.emit_rex64_modrm_address(dest, src);
        self.emit_u8(0x8D);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x89);
        self.emit_modrm_registers(src, dest);
    }

    pub fn movq_ri(&mut self, dest: Register, imm: Immediate) {
        if imm.is_int32() {
            self.emit_rex64_rm(dest);
            self.emit_u8(0xC7);
            self.emit_modrm_opcode(0, dest);
            self.emit_u32(imm.int32() as u32);
        } else {
            self.emit_rex64_rm(dest);
            self.emit_u8(0xB8 + dest.low_bits());
            self.emit_u64(imm.int64() as u64);
        }
    }

    pub fn movq_ra(&mut self, dest: Register, src: Address) {
        self.emit_rex64_modrm_address(dest, src);
        self.emit_u8(0x8B);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movb_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex32_byte_address(src, dest);
        self.emit_u8(0x88);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn movb_ai(&mut self, dest: Address, src: Immediate) {
        assert!(src.is_int8() || src.is_uint8());
        self.emit_rex32_address_optional(dest);
        self.emit_u8(0xc6);
        self.emit_address(0b000, dest);
        self.emit_u8(src.uint8());
    }

    pub fn movq_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x89);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn movq_ai(&mut self, dest: Address, imm: Immediate) {
        assert!(imm.is_int32());
        self.emit_rex64_address(dest);
        self.emit_u8(0xc7);
        self.emit_address(0b000, dest);
        self.emit_u32(imm.int32() as u32);
    }

    pub fn movl_ra(&mut self, dest: Register, src: Address) {
        self.emit_rex32_modrm_address(dest, src);
        self.emit_u8(0x8B);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movl_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x89);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn movl_ai(&mut self, dest: Address, imm: Immediate) {
        assert!(imm.is_int32() || imm.is_uint32());
        self.emit_rex32_address_optional(dest);
        self.emit_u8(0xc7);
        self.emit_address(0b000, dest);
        self.emit_u32(imm.uint32());
    }

    pub fn movl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x89);
        self.emit_modrm_registers(src, dest);
    }

    pub fn movl_ri(&mut self, dest: Register, imm: Immediate) {
        assert!(imm.is_int32());
        self.emit_rex32_rm_optional(dest);
        self.emit_u8(0xB8 + dest.low_bits());
        self.emit_u32(imm.int32() as u32);
    }

    pub fn movzxb_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_byte_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xb6);
        self.emit_modrm_registers(dest, src);
    }

    pub fn movzxb_ra(&mut self, dest: Register, src: Address) {
        self.emit_rex32_modrm_address(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xb6);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movsxbl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_byte_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbe);
        self.emit_modrm_registers(dest, src);
    }

    pub fn movsxbl_ra(&mut self, dest: Register, src: Address) {
        self.emit_rex32_modrm_address(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbe);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movsxbq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbe);
        self.emit_modrm_registers(dest, src);
    }

    pub fn movsxbq_ra(&mut self, dest: Register, src: Address) {
        self.emit_rex64_modrm_address(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbe);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movsxlq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x63);
        self.emit_modrm_registers(dest, src);
    }

    pub fn movss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x10);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn movss_ra(&mut self, dest: XmmRegister, src: Address) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_address_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x10);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movss_ar(&mut self, dest: Address, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_address_optional(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0x11);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn movsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x10);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn movsd_ra(&mut self, dest: XmmRegister, src: Address) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_address_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x10);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn movsd_ar(&mut self, dest: Address, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_address_optional(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0x11);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn addq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x01);
        self.emit_modrm_registers(src, dest);
    }

    pub fn addl_ri(&mut self, dest: Register, imm: Immediate) {
        self.emit_alu32_imm(dest, imm, 0b000, 0x05);
    }

    pub fn addq_ri(&mut self, dest: Register, imm: Immediate) {
        self.emit_alu64_imm(dest, imm, 0b000, 0x05);
    }

    pub fn addl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x01);
        self.emit_modrm_registers(src, dest);
    }

    pub fn addss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x58);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn addsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x58);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn subq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x29);
        self.emit_modrm_registers(src, dest);
    }

    pub fn subq_ri(&mut self, dest: Register, imm: Immediate) {
        self.emit_alu64_imm(dest, imm, 0b101, 0x2D);
    }

    pub fn subq_ri32(&mut self, dest: Register, imm: Immediate) {
        self.emit_alu64_imm(dest, imm, 0b101, 0x2D);
    }

    pub fn subl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x29);
        self.emit_modrm_registers(src, dest);
    }

    pub fn subss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5c);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn subsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5c);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn andl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x21);
        self.emit_modrm_registers(src, dest);
    }

    pub fn andq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x21);
        self.emit_modrm_registers(src, dest);
    }

    pub fn andq_ri(&mut self, dest: Register, imm: Immediate) {
        self.emit_alu64_imm(dest, imm, 0b100, 0x25);
    }

    pub fn cmpb_ar(&mut self, lhs: Address, rhs: Register) {
        self.emit_rex32_byte_address(rhs, lhs);
        self.emit_u8(0x38);
        self.emit_address(rhs.low_bits(), lhs);
    }

    pub fn cmpb_ai(&mut self, lhs: Address, rhs: Immediate) {
        assert!(rhs.is_int8() || rhs.is_uint8());
        self.emit_rex32_address_optional(lhs);
        self.emit_u8(0x80);
        self.emit_address(0b111, lhs);
        self.emit_u8(rhs.uint8());
    }

    pub fn cmpl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x39);
        self.emit_modrm_registers(src, dest);
    }

    pub fn cmpl_ar(&mut self, lhs: Address, rhs: Register) {
        self.emit_rex32_modrm_address(rhs, lhs);
        self.emit_u8(0x39);
        self.emit_address(rhs.low_bits(), lhs);
    }

    pub fn cmpl_ai(&mut self, lhs: Address, rhs: Immediate) {
        assert!(rhs.is_int32() || rhs.is_uint32());

        if rhs.is_int8() {
            self.emit_rex32_address_optional(lhs);
            self.emit_u8(0x83);
            self.emit_address(0b111, lhs);
            self.emit_u8(rhs.int8() as u8);
        } else {
            self.emit_rex32_address_optional(lhs);
            self.emit_u8(0x81);
            self.emit_address(0b111, lhs);
            self.emit_u32(rhs.uint32());
        }
    }

    pub fn cmpq_rr(&mut self, lhs: Register, rhs: Register) {
        self.emit_rex64_modrm(rhs, lhs);
        self.emit_u8(0x39);
        self.emit_modrm_registers(rhs, lhs);
    }

    pub fn cmpq_ar(&mut self, lhs: Address, rhs: Register) {
        self.emit_rex64_modrm_address(rhs, lhs);
        self.emit_u8(0x39);
        self.emit_address(rhs.low_bits(), lhs);
    }

    pub fn cmpq_ai(&mut self, lhs: Address, rhs: Immediate) {
        assert!(rhs.is_int32());

        if rhs.is_int8() {
            self.emit_rex64_address(lhs);
            self.emit_u8(0x83);
            self.emit_address(0b111, lhs);
            self.emit_u8(rhs.int8() as u8);
        } else {
            self.emit_rex64_address(lhs);
            self.emit_u8(0x81);
            self.emit_address(0b111, lhs);
            self.emit_u32(rhs.int32() as u32);
        }
    }

    pub fn cmpq_ri(&mut self, reg: Register, imm: Immediate) {
        self.emit_alu64_imm(reg, imm, 0b111, 0x3d);
    }

    pub fn cmpl_ri(&mut self, reg: Register, imm: Immediate) {
        self.emit_alu32_imm(reg, imm, 0b111, 0x3d);
    }

    pub fn ucomiss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x2e);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn ucomisd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0x66);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x2e);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn sqrtss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x51);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn sqrtsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x51);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn pxor_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0x66);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xef);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn orl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x09);
        self.emit_modrm_registers(src, dest);
    }

    pub fn orq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x09);
        self.emit_modrm_registers(src, dest);
    }

    pub fn xchgq_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x87);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn xchgl_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x87);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn cmpxchgq_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xb1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn cmpxchgl_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xb1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn lock_cmpxchgq_ar(&mut self, dest: Address, src: Register) {
        self.emit_lock_prefix();
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xb1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn lock_cmpxchgl_ar(&mut self, dest: Address, src: Register) {
        self.emit_lock_prefix();
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xb1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn xorl_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(src, dest);
        self.emit_u8(0x31);
        self.emit_modrm_registers(src, dest);
    }

    pub fn xorl_ri(&mut self, lhs: Register, rhs: Immediate) {
        self.emit_alu32_imm(lhs, rhs, 0b110, 0x35);
    }

    pub fn xorq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(src, dest);
        self.emit_u8(0x31);
        self.emit_modrm_registers(src, dest);
    }

    pub fn xorps_ra(&mut self, dest: XmmRegister, src: Address) {
        self.emit_rex_sse_address_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x57);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn xorps_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x57);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn xorpd_ra(&mut self, dest: XmmRegister, src: Address) {
        self.emit_u8(0x66);
        self.emit_rex_sse_address_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x57);
        self.emit_address(dest.low_bits(), src);
    }

    pub fn testl_rr(&mut self, lhs: Register, rhs: Register) {
        self.emit_rex32_optional(rhs, lhs);
        self.emit_u8(0x85);
        self.emit_modrm_registers(rhs, lhs);
    }

    pub fn testl_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int32());

        if rhs.is_uint8() {
            if lhs == RAX {
                self.emit_u8(0xa8);
            } else if lhs.value() < 4 {
                self.emit_u8(0xf6);
                self.emit_modrm_opcode(0b000, lhs);
            } else {
                self.emit_rex(false, false, false, lhs.needs_rex());
                self.emit_u8(0xf6);
                self.emit_modrm_opcode(0b000, lhs);
            }
            self.emit_u8(rhs.uint8());
        } else if lhs == RAX {
            self.emit_u8(0xa9);
            self.emit_u32(rhs.int32() as u32);
        } else {
            self.emit_u8(0xf7);
            self.emit_modrm_opcode(0b000, lhs);
            self.emit_u32(rhs.int32() as u32);
        }
    }

    pub fn testl_ar(&mut self, lhs: Address, rhs: Register) {
        self.emit_rex32_modrm_address(rhs, lhs);
        self.emit_u8(0x85);
        self.emit_address(rhs.low_bits(), lhs);
    }

    pub fn testl_ai(&mut self, lhs: Address, rhs: Immediate) {
        assert!(rhs.is_int32());
        self.emit_rex32_address_optional(lhs);
        self.emit_u8(0xf7);
        self.emit_address(0b000, lhs);
        self.emit_u32(rhs.int32() as u32);
    }

    pub fn testq_rr(&mut self, lhs: Register, rhs: Register) {
        self.emit_rex64_modrm(rhs, lhs);
        self.emit_u8(0x85);
        self.emit_modrm_registers(rhs, lhs);
    }

    pub fn testq_ar(&mut self, lhs: Address, rhs: Register) {
        self.emit_rex64_modrm_address(rhs, lhs);
        self.emit_u8(0x85);
        self.emit_address(rhs.low_bits(), lhs);
    }

    pub fn testq_ai(&mut self, lhs: Address, rhs: Immediate) {
        assert!(rhs.is_int32());
        self.emit_rex64_address(lhs);
        self.emit_u8(0xf7);
        self.emit_address(0b000, lhs);
        self.emit_u32(rhs.int32() as u32);
    }

    pub fn imull_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex32_optional(dest, src);
        self.emit_u8(0x0F);
        self.emit_u8(0xAF);
        self.emit_modrm_registers(dest, src);
    }

    pub fn imulq_rr(&mut self, dest: Register, src: Register) {
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0F);
        self.emit_u8(0xAF);
        self.emit_modrm_registers(dest, src);
    }

    pub fn mulss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x59);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn mulsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x59);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn divss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5e);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn divsd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5e);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn idivl_r(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b111, reg);
    }

    pub fn idivq_r(&mut self, src: Register) {
        self.emit_rex64_rm(src);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b111, src);
    }

    pub fn call_r(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0xFF);
        self.emit_modrm_opcode(0b010, reg);
    }

    pub fn cdq(&mut self) {
        self.emit_u8(0x99);
    }

    pub fn cqo(&mut self) {
        self.emit_rex64();
        self.emit_u8(0x99);
    }

    pub fn negl(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b011, reg);
    }

    pub fn negq(&mut self, reg: Register) {
        self.emit_rex64_rm(reg);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b011, reg);
    }

    pub fn notl(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b010, reg);
    }

    pub fn notq(&mut self, reg: Register) {
        self.emit_rex64_rm(reg);
        self.emit_u8(0xF7);
        self.emit_modrm_opcode(0b010, reg);
    }

    pub fn jcc(&mut self, condition: Condition, target: Label) {
        if let Some(target_offset) = self.offset(target) {
            // backwards jump
            // rip = end of current instruction = pc + 2
            let target_offset = target_offset as usize;
            assert!(target_offset <= self.position());
            let distance = self.position() + 2 - target_offset;
            let distance = -(distance as isize);
            assert!(distance <= -2);

            if distance >= -128 {
                self.emit_u8(0x70 + condition.int());
                self.emit_u8(distance as u8);
            } else {
                let distance = self.position() + 6 - target_offset;
                let distance = -(distance as isize);
                self.emit_u8(0x0F);
                self.emit_u8(0x80 + condition.int());
                self.emit_u32(distance as u32);
            }
        } else {
            // forward jump - conservatively assume far jump
            self.emit_u8(0x0F);
            self.emit_u8(0x80 + condition.int());
            self.unresolved_jumps.push(ForwardJump {
                offset: self.position().try_into().unwrap(),
                label: target,
                distance: JumpDistance::Far,
            });
            self.emit_u32(0);
        }
    }

    pub fn jcc_near(&mut self, condition: Condition, target: Label) {
        if let Some(target_offset) = self.offset(target) {
            // backwards jump
            // rip = end of current instruction = pc + 2
            let target_offset = target_offset as usize;
            assert!(target_offset <= self.position());
            let distance = self.position() + 2 - target_offset;
            let distance = -(distance as isize);
            assert!(-128 <= distance && distance <= -2);
            self.emit_u8(0x70 + condition.int());
            self.emit_u8(distance as u8);
        } else {
            // forward jump
            self.emit_u8(0x70 + condition.int());
            self.unresolved_jumps.push(ForwardJump {
                offset: self.position().try_into().unwrap(),
                label: target,
                distance: JumpDistance::Near,
            });
            self.emit_u8(0);
        }
    }

    pub fn jmp(&mut self, target: Label) {
        if let Some(target_offset) = self.offset(target) {
            // backwards jump
            // rip = end of current instruction = pc + 2
            let target_offset = target_offset as usize;
            assert!(target_offset <= self.position());
            let distance = self.position() + 2 - target_offset;
            let distance = -(distance as isize);
            assert!(distance <= -2);

            if distance >= -128 {
                self.emit_u8(0xEB);
                self.emit_u8(distance as u8);
            } else {
                let distance = self.position() + 5 - target_offset;
                let distance = -(distance as isize);
                self.emit_u8(0xE9);
                self.emit_u32(distance as u32);
            }
        } else {
            // forward jump - conservatively assume far jump
            self.emit_u8(0xE9);
            self.unresolved_jumps.push(ForwardJump {
                offset: self.position().try_into().unwrap(),
                label: target,
                distance: JumpDistance::Far,
            });
            self.emit_u32(0);
        }
    }

    pub fn jmp_near(&mut self, target: Label) {
        if let Some(target_offset) = self.offset(target) {
            // backwards jump
            // rip = end of current instruction = pc + 2
            let target_offset = target_offset as usize;
            assert!(target_offset <= self.position());
            let distance = self.position() + 2 - target_offset;
            let distance = -(distance as isize);
            assert!(-128 <= distance && distance <= -2);
            self.emit_u8(0xEB);
            self.emit_u8(distance as u8);
        } else {
            // forward jump - conservatively assume far jump
            self.emit_u8(0xEB);
            self.unresolved_jumps.push(ForwardJump {
                offset: self.position().try_into().unwrap(),
                label: target,
                distance: JumpDistance::Near,
            });
            self.emit_u8(0);
        }
    }

    pub fn jmp_r(&mut self, reg: Register) {
        self.emit_rex32_rm_optional(reg);
        self.emit_u8(0xff);
        self.emit_modrm_opcode(0b100, reg);
    }

    pub fn tzcntl_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex32_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbc);
        self.emit_modrm_registers(dest, src);
    }

    pub fn tzcntq_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbc);
        self.emit_modrm_registers(dest, src);
    }

    pub fn lzcntl_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex32_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbd);
        self.emit_modrm_registers(dest, src);
    }

    pub fn lzcntq_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xbd);
        self.emit_modrm_registers(dest, src);
    }

    pub fn popcntl_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex32_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xb8);
        self.emit_modrm_registers(dest, src);
    }

    pub fn popcntq_rr(&mut self, dest: Register, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex64_modrm(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0xb8);
        self.emit_modrm_registers(dest, src);
    }

    pub fn cvtsd2ss_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5a);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn cvtss2sd_rr(&mut self, dest: XmmRegister, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_sse_modrm_optional(dest, src);
        self.emit_u8(0x0f);
        self.emit_u8(0x5a);
        self.emit_modrm_sse_registers(dest, src);
    }

    pub fn cvtsi2ssd_rr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex_optional(false, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2a);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvtsi2ssq_rr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0xf3);
        self.emit_rex_optional(true, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2a);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvtsi2sdd_rr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0xf2);
        self.emit_rex_optional(false, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2a);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvtsi2sdq_rr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0xf2);
        self.emit_rex_optional(true, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2a);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvttss2sid_rr(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_optional(false, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2c);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvttss2siq_rr(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0xf3);
        self.emit_rex_optional(true, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2c);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvttsd2sid_rr(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_optional(false, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2c);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn cvttsd2siq_rr(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0xf2);
        self.emit_rex_optional(true, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x2c);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn movd_rx(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0x66);
        self.emit_rex_optional(false, src.needs_rex(), false, dest.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x7e);
        self.emit_modrm(0b11, src.low_bits(), dest.low_bits());
    }

    pub fn movd_xr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0x66);
        self.emit_rex_optional(false, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x6e);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn movq_rx(&mut self, dest: Register, src: XmmRegister) {
        self.emit_u8(0x66);
        self.emit_rex_optional(true, src.needs_rex(), false, dest.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x7e);
        self.emit_modrm(0b11, src.low_bits(), dest.low_bits());
    }

    pub fn movq_xr(&mut self, dest: XmmRegister, src: Register) {
        self.emit_u8(0x66);
        self.emit_rex_optional(true, dest.needs_rex(), false, src.needs_rex());
        self.emit_u8(0x0f);
        self.emit_u8(0x6e);
        self.emit_modrm(0b11, dest.low_bits(), src.low_bits());
    }

    pub fn shrl_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b101, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn shrl_r(&mut self, lhs: Register) {
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b101, lhs);
    }

    pub fn shll_r(&mut self, lhs: Register) {
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b100, lhs);
    }

    pub fn sarl_r(&mut self, lhs: Register) {
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b111, lhs);
    }

    pub fn shrq_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b101, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn shrq_r(&mut self, lhs: Register) {
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b101, lhs);
    }

    pub fn sarq_r(&mut self, lhs: Register) {
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b111, lhs);
    }

    pub fn sarl_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b111, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn sarq_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b111, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn shll_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex32_rm_optional(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b100, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn shlq_r(&mut self, lhs: Register) {
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b100, lhs);
    }

    pub fn shlq_ri(&mut self, lhs: Register, rhs: Immediate) {
        assert!(rhs.is_int8());
        self.emit_rex64_rm(lhs);
        self.emit_u8(0xc1);
        self.emit_modrm_opcode(0b100, lhs);
        self.emit_u8(rhs.int8() as u8);
    }

    pub fn roll_r(&mut self, opnd: Register) {
        self.emit_rex32_rm_optional(opnd);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b000, opnd);
    }

    pub fn rolq_r(&mut self, opnd: Register) {
        self.emit_rex64_rm(opnd);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b000, opnd);
    }

    pub fn rorl_r(&mut self, opnd: Register) {
        self.emit_rex32_rm_optional(opnd);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b001, opnd);
    }

    pub fn rorq_r(&mut self, opnd: Register) {
        self.emit_rex64_rm(opnd);
        self.emit_u8(0xd3);
        self.emit_modrm_opcode(0b001, opnd);
    }

    pub fn xaddq_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xc1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn xaddl_ar(&mut self, dest: Address, src: Register) {
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xc1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn lock_xaddq_ar(&mut self, dest: Address, src: Register) {
        self.emit_lock_prefix();
        self.emit_rex64_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xc1);
        self.emit_address(src.low_bits(), dest);
    }

    pub fn lock_xaddl_ar(&mut self, dest: Address, src: Register) {
        self.emit_lock_prefix();
        self.emit_rex32_modrm_address(src, dest);
        self.emit_u8(0x0f);
        self.emit_u8(0xc1);
        self.emit_address(src.low_bits(), dest);
    }

    fn emit_lock_prefix(&mut self) {
        self.emit_u8(0xF0);
    }

    fn emit_rex_sse_modrm_optional(&mut self, reg: XmmRegister, rm: XmmRegister) {
        if reg.needs_rex() || rm.needs_rex() {
            self.emit_rex(false, reg.needs_rex(), false, rm.needs_rex());
        }
    }

    fn emit_rex_sse_address_optional(&mut self, reg: XmmRegister, address: Address) {
        if address.rex != 0 || reg.needs_rex() {
            self.emit_u8(0x40 | address.rex | if reg.needs_rex() { 0x04 } else { 0 });
        }
    }

    fn emit_rex32_rm_optional(&mut self, reg: Register) {
        if reg.needs_rex() {
            self.emit_rex(false, false, false, true);
        }
    }

    fn emit_rex32_byte_optional(&mut self, reg: Register, rm: Register) {
        if reg.needs_rex() || rm.needs_rex() || rm.value() > 3 {
            self.emit_rex(false, reg.needs_rex(), false, rm.needs_rex());
        }
    }

    fn emit_rex32_optional(&mut self, reg: Register, rm: Register) {
        if reg.needs_rex() || rm.needs_rex() {
            self.emit_rex(false, reg.needs_rex(), false, rm.needs_rex());
        }
    }

    fn emit_rex64(&mut self) {
        self.emit_rex(true, false, false, false);
    }

    fn emit_rex64_rm(&mut self, rm: Register) {
        self.emit_rex(true, false, false, rm.needs_rex());
    }

    fn emit_rex64_modrm_address(&mut self, reg: Register, address: Address) {
        let rex = 0x48 | address.rex | if reg.needs_rex() { 0x04 } else { 0 };
        self.emit_u8(rex);
    }

    fn emit_rex64_address(&mut self, address: Address) {
        self.emit_u8(0x48 | address.rex);
    }

    fn emit_rex32_modrm_address(&mut self, reg: Register, address: Address) {
        if address.rex != 0 || reg.needs_rex() {
            self.emit_u8(0x40 | address.rex | if reg.needs_rex() { 0x04 } else { 0 });
        }
    }

    fn emit_rex32_byte_address(&mut self, reg: Register, address: Address) {
        if address.rex != 0 || reg.value() > 3 {
            self.emit_u8(0x40 | address.rex | if reg.needs_rex() { 0x04 } else { 0 });
        }
    }

    fn emit_rex32_address_optional(&mut self, address: Address) {
        if address.rex != 0 {
            self.emit_u8(0x40 | address.rex);
        }
    }

    fn emit_rex64_modrm(&mut self, reg: Register, rm: Register) {
        self.emit_rex(true, reg.needs_rex(), false, rm.needs_rex());
    }

    fn emit_rex(&mut self, w: bool, r: bool, x: bool, b: bool) {
        // w - 64-bit width
        // r - extension of modrm-reg field
        // x - extension of sib index field
        // b - extension of modrm-rm/sib base/opcode reg field
        let opcode = 0x40 | (w as u8) << 3 | (r as u8) << 2 | (x as u8) << 1 | b as u8;
        self.emit_u8(opcode);
    }

    fn emit_rex_optional(&mut self, w: bool, r: bool, x: bool, b: bool) {
        if w || r || x || b {
            self.emit_rex(w, r, x, b);
        }
    }

    fn emit_modrm_registers(&mut self, reg: Register, rm: Register) {
        self.emit_modrm(0b11, reg.low_bits(), rm.low_bits());
    }

    fn emit_modrm_sse_registers(&mut self, reg: XmmRegister, rm: XmmRegister) {
        self.emit_modrm(0b11, reg.low_bits(), rm.low_bits());
    }

    fn emit_modrm_opcode(&mut self, opcode: u8, reg: Register) {
        self.emit_modrm(0b11, opcode, reg.low_bits());
    }

    fn emit_modrm(&mut self, mode: u8, reg: u8, rm: u8) {
        assert!(mode < 4);
        assert!(reg < 8);
        assert!(rm < 8);
        self.emit_u8(mode << 6 | reg << 3 | rm);
    }

    fn emit_address(&mut self, reg_or_opcode: u8, address: Address) {
        assert!(reg_or_opcode < 8);

        let bytes = address.encoded_bytes();

        // emit modrm-byte with the given rm value
        self.emit_u8(reg_or_opcode << 3 | bytes[0]);

        for &byte in &bytes[1..] {
            self.emit_u8(byte);
        }
    }

    fn emit_alu64_imm(&mut self, reg: Register, imm: Immediate, modrm_reg: u8, rax_opcode: u8) {
        assert!(imm.is_int32());
        self.emit_rex64_rm(reg);

        if imm.is_int8() {
            self.emit_u8(0x83);
            self.emit_modrm_opcode(modrm_reg, reg);
            self.emit_u8(imm.int8() as u8);
        } else if reg == RAX {
            self.emit_u8(rax_opcode);
            self.emit_u32(imm.int32() as u32);
        } else {
            self.emit_u8(0x81);
            self.emit_modrm_opcode(modrm_reg, reg);
            self.emit_u32(imm.int32() as u32);
        }
    }

    fn emit_alu32_imm(&mut self, reg: Register, imm: Immediate, modrm_reg: u8, rax_opcode: u8) {
        assert!(imm.is_int32());
        self.emit_rex32_rm_optional(reg);

        if imm.is_int8() {
            self.emit_u8(0x83);
            self.emit_modrm_opcode(modrm_reg, reg);
            self.emit_u8(imm.int8() as u8);
        } else if reg == RAX {
            self.emit_u8(rax_opcode);
            self.emit_u32(imm.int32() as u32);
        } else {
            self.emit_u8(0x81);
            self.emit_modrm_opcode(modrm_reg, reg);
            self.emit_u32(imm.int32() as u32);
        }
    }
}

#[derive(Copy, Clone)]
pub enum Condition {
    Overflow,
    NoOverflow,
    Below,
    NeitherAboveNorEqual,
    NotBelow,
    AboveOrEqual,
    Equal,
    Zero,
    NotEqual,
    NotZero,
    BelowOrEqual,
    NotAbove,
    NeitherBelowNorEqual,
    Above,
    Sign,
    NoSign,
    Parity,
    ParityEven,
    NoParity,
    ParityOdd,
    Less,
    NeitherGreaterNorEqual,
    NotLess,
    GreaterOrEqual,
    LessOrEqual,
    NotGreater,
    NeitherLessNorEqual,
    Greater,
}

impl Condition {
    pub fn int(self) -> u8 {
        match self {
            Condition::Overflow => 0b0000,
            Condition::NoOverflow => 0b0001,
            Condition::Below | Condition::NeitherAboveNorEqual => 0b0010,
            Condition::NotBelow | Condition::AboveOrEqual => 0b0011,
            Condition::Equal | Condition::Zero => 0b0100,
            Condition::NotEqual | Condition::NotZero => 0b0101,
            Condition::BelowOrEqual | Condition::NotAbove => 0b0110,
            Condition::NeitherBelowNorEqual | Condition::Above => 0b0111,
            Condition::Sign => 0b1000,
            Condition::NoSign => 0b1001,
            Condition::Parity | Condition::ParityEven => 0b1010,
            Condition::NoParity | Condition::ParityOdd => 0b1011,
            Condition::Less | Condition::NeitherGreaterNorEqual => 0b1100,
            Condition::NotLess | Condition::GreaterOrEqual => 0b1101,
            Condition::LessOrEqual | Condition::NotGreater => 0b1110,
            Condition::NeitherLessNorEqual | Condition::Greater => 0b1111,
        }
    }
}

pub struct Immediate(pub i64);

impl Immediate {
    pub fn is_int8(&self) -> bool {
        let limit = 1i64 << 7;
        -limit <= self.0 && self.0 < limit
    }

    pub fn is_int32(&self) -> bool {
        let limit = 1i64 << 31;
        -limit <= self.0 && self.0 < limit
    }

    pub fn is_uint8(&self) -> bool {
        0 <= self.0 && self.0 < 256
    }

    pub fn is_uint32(&self) -> bool {
        let limit = 1i64 << 32;
        0 <= self.0 && self.0 < limit
    }

    pub fn uint8(&self) -> u8 {
        self.0 as u8
    }

    pub fn int8(&self) -> i8 {
        self.0 as i8
    }

    pub fn int32(&self) -> i32 {
        self.0 as i32
    }

    pub fn uint32(&self) -> u32 {
        self.0 as u32
    }

    pub fn int64(&self) -> i64 {
        self.0
    }
}

#[derive(Copy, Clone)]
pub struct XmmRegister(u8);

impl XmmRegister {
    pub fn new(value: u8) -> XmmRegister {
        XmmRegister(value)
    }

    pub fn low_bits(self) -> u8 {
        self.0 & 0b111
    }

    pub fn value(self) -> u8 {
        self.0
    }

    pub fn needs_rex(self) -> bool {
        self.0 > 7
    }
}

#[derive(Copy, Clone)]
pub enum ScaleFactor {
    One,
    Two,
    Four,
    Eight,
}

impl ScaleFactor {
    fn value(self) -> u8 {
        match self {
            ScaleFactor::One => 0,
            ScaleFactor::Two => 1,
            ScaleFactor::Four => 2,
            ScaleFactor::Eight => 3,
        }
    }
}

#[derive(Copy, Clone)]
pub struct Address {
    rex: u8,
    length: u8,
    bytes: [u8; 6],
}

impl Address {
    fn new() -> Address {
        Address {
            rex: 0,
            length: 0,
            bytes: [0; 6],
        }
    }

    fn set_modrm(&mut self, mode: u8, reg: Register) {
        assert!(mode < 4);
        assert_eq!(self.length, 0);

        if reg.needs_rex() {
            self.rex |= 0x41;
        }

        self.bytes[0] = mode << 6 | reg.low_bits();
        self.length += 1;
    }

    fn set_sib(&mut self, scale: ScaleFactor, index: Register, base: Register) {
        assert_eq!(self.length, 1);

        if base.needs_rex() {
            self.rex |= 0x41;
        }

        if index.needs_rex() {
            self.rex |= 0x42;
        }

        self.bytes[1] = scale.value() << 6 | index.low_bits() << 3 | base.low_bits();
        self.length += 1;
    }

    fn set_disp8(&mut self, imm: i8) {
        assert!(self.length == 1 || self.length == 2);
        self.bytes[self.length as usize] = imm as u8;
        self.length += 1;
    }

    fn set_disp32(&mut self, imm: i32) {
        assert!(self.length == 1 || self.length == 2);
        let idx = self.length as usize;
        let imm = imm as u32;
        self.bytes[idx] = imm as u8;
        self.bytes[idx + 1] = (imm >> 8) as u8;
        self.bytes[idx + 2] = (imm >> 16) as u8;
        self.bytes[idx + 3] = (imm >> 24) as u8;
        self.length += 4;
    }

    pub fn reg(base: Register) -> Address {
        Address::offset(base, 0)
    }

    pub fn offset(base: Register, offset: i32) -> Address {
        let mut address = Address::new();

        let mode = if offset == 0 && base != RBP {
            0b00
        } else if -128 <= offset && offset < 128 {
            0b01
        } else {
            0b10
        };

        address.set_modrm(mode, base);

        if base == RSP {
            address.set_sib(ScaleFactor::One, RSP, base);
        }

        match mode {
            0b00 => {}
            0b01 => address.set_disp8(offset as i8),
            0b10 => address.set_disp32(offset),
            _ => unreachable!(),
        }

        address
    }

    pub fn index(index: Register, factor: ScaleFactor, disp: i32) -> Address {
        let mut address = Address::new();

        address.set_modrm(0b00, RSP);
        assert_ne!(index, RSP);

        address.set_sib(factor, index, RBP);
        address.set_disp32(disp);

        address
    }

    pub fn array(base: Register, index: Register, factor: ScaleFactor, disp: i32) -> Address {
        let mut address = Address::new();

        let mode = if disp == 0 && base != RBP {
            0b00
        } else if -128 <= disp && disp < 128 {
            0b01
        } else {
            0b10
        };

        address.set_modrm(mode, RSP);
        assert_ne!(index, RSP);

        address.set_sib(factor, index, base);

        match mode {
            0b00 => {}
            0b01 => address.set_disp8(disp as i8),
            0b10 => address.set_disp32(disp),
            _ => unreachable!(),
        }

        address
    }

    pub fn rip(disp: i32) -> Address {
        let mut address = Address::new();

        address.set_modrm(0b00, RBP);
        address.set_disp32(disp);

        address
    }

    pub fn encoded_bytes(&self) -> &[u8] {
        &self.bytes[0..self.length as usize]
    }
}
