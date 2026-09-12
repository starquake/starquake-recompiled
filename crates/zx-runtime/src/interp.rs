//! Fallback interpreter.
//!
//! Runs whatever the recompiler could not see statically: code reached only
//! through computed jumps that were never traced, self-modifying code, and
//! the recompiler's own trace pass. It uses the same decoder and ALU helpers
//! as the generated code, so the two agree instruction for instruction.

use crate::machine::Zx;
use zx_core::{Addr, Decoded, Instr, Op8, decode};

pub fn decode_at(z: &Zx, pc: u16) -> Decoded {
    let mem = &z.mem;
    decode(&|a| mem[a as usize], pc)
}

fn addr(z: &Zx, a: Addr) -> u16 {
    match a {
        Addr::BC => z.bc(),
        Addr::DE => z.de(),
        Addr::HL => z.hl(),
        Addr::Idx(i, d) => z.r16(i.reg16()).wrapping_add(d as i16 as u16),
        Addr::Abs(nn) => nn,
    }
}

fn get8(z: &Zx, o: Op8) -> u8 {
    match o {
        Op8::Reg(r) => z.r8(r),
        Op8::Mem(a) => z.read(addr(z, a)),
        Op8::Imm(n) => n,
    }
}

fn set8(z: &mut Zx, o: Op8, v: u8) {
    match o {
        Op8::Reg(r) => z.set_r8(r, v),
        Op8::Mem(a) => z.write(addr(z, a), v),
        Op8::Imm(_) => unreachable!("store to immediate"),
    }
}

/// Executes one instruction at `z.pc`.
pub fn step(z: &mut Zx) {
    let pc = z.pc;
    let d = decode_at(z, pc);
    let next = pc.wrapping_add(d.len as u16);
    if let Some(trace) = &mut z.trace {
        trace.on_exec(pc, &z.mem, d.len);
    }
    z.pc = next;
    z.step(d.t, d.m1);
    execute(z, &d, pc, next);
}

fn execute(z: &mut Zx, d: &Decoded, pc: u16, next: u16) {
    use Instr::*;
    let cond = |z: &Zx, c: Option<zx_core::Cond>| c.is_none_or(|c| z.cond(c));
    match d.instr {
        Nop => {}
        // The processor holds PC on the HALT and keeps re-fetching it;
        // accepting an interrupt is what steps past it.
        Halt => {
            z.halted = true;
            z.pc = pc;
        }
        Di => z.di(),
        Ei => {
            z.ei();
            z.ei_delay = true;
        }
        Ld8(dst, src) => {
            let v = get8(z, src);
            set8(z, dst, v);
        }
        Ld16(r, nn) => z.set_r16(r, nn),
        Ld16Load(r, a) => {
            let v = z.read16(a);
            z.set_r16(r, v);
        }
        Ld16Store(a, r) => z.write16(a, z.r16(r)),
        LdSp(r) => z.sp = z.r16(r),
        Push(r) => z.push(z.r16(r)),
        Pop(r) => {
            let v = z.pop();
            z.set_r16(r, v);
        }
        ExDeHl => z.ex_de_hl(),
        ExAf => z.ex_af(),
        Exx => z.exx(),
        ExSp(r) => {
            let v = z.read16(z.sp);
            z.write16(z.sp, z.r16(r));
            z.set_r16(r, v);
        }
        Alu(op, src) => {
            let v = get8(z, src);
            z.alu(op, v);
        }
        Inc8(o) => {
            let v = z.inc8(get8(z, o));
            set8(z, o, v);
        }
        Dec8(o) => {
            let v = z.dec8(get8(z, o));
            set8(z, o, v);
        }
        Inc16(r) => z.set_r16(r, z.r16(r).wrapping_add(1)),
        Dec16(r) => z.set_r16(r, z.r16(r).wrapping_sub(1)),
        Add16(dst, src) => {
            let v = z.add16(z.r16(dst), z.r16(src));
            z.set_r16(dst, v);
        }
        Adc16(r) => z.adc_hl(z.r16(r)),
        Sbc16(r) => z.sbc_hl(z.r16(r)),
        Daa => z.daa(),
        Cpl => z.cpl(),
        Neg => z.neg(),
        Ccf => z.ccf(),
        Scf => z.scf(),
        Rlca => z.rlca(),
        Rrca => z.rrca(),
        Rla => z.rla(),
        Rra => z.rra(),
        Rld => z.rld(),
        Rrd => z.rrd(),
        Rot(op, o, copy) => {
            let v = z.rot(op, get8(z, o));
            set8(z, o, v);
            if let Some(r) = copy {
                z.set_r8(r, v);
            }
        }
        Bit(n, o) => {
            let v = get8(z, o);
            // Undocumented: for the indexed form, flag bits 3 and 5 come
            // from the high byte of the address rather than the byte read.
            let undocumented = match o {
                Op8::Mem(a @ Addr::Idx(..)) => (addr(z, a) >> 8) as u8,
                _ => v,
            };
            z.bit(n, v, undocumented);
        }
        Res(n, o, copy) | Set(n, o, copy) => {
            let v = get8(z, o);
            let v = if matches!(d.instr, Res(..)) { v & !(1 << n) } else { v | (1 << n) };
            set8(z, o, v);
            if let Some(r) = copy {
                z.set_r8(r, v);
            }
        }
        Jp(c, a) => {
            if cond(z, c) {
                z.pc = a;
            }
        }
        JpInd(r) => z.pc = z.r16(r),
        Jr(c, a) => {
            if cond(z, c) {
                z.t += d.t_extra as u32;
                z.pc = a;
            }
        }
        Djnz(a) => {
            z.b = z.b.wrapping_sub(1);
            if z.b != 0 {
                z.t += d.t_extra as u32;
                z.pc = a;
            }
        }
        Call(c, a) => {
            if cond(z, c) {
                z.t += d.t_extra as u32;
                z.push(next);
                z.pc = a;
            }
        }
        Ret(c) => {
            if cond(z, c) {
                z.t += d.t_extra as u32;
                z.pc = z.pop();
            }
        }
        Reti | Retn => {
            z.iff1 = z.iff2;
            z.pc = z.pop();
        }
        Rst(n) => {
            z.push(next);
            z.pc = n as u16;
        }
        InA(n) => z.a = z.port_in((z.a as u16) << 8 | n as u16),
        InC(r) => {
            let v = z.port_in(z.bc());
            z.in_flags(v);
            if let Some(r) = r {
                z.set_r8(r, v);
            }
        }
        OutA(n) => z.port_out((z.a as u16) << 8 | n as u16, z.a),
        OutC(r) => {
            let v = r.map_or(0, |r| z.r8(r));
            z.port_out(z.bc(), v);
        }
        Block(op) => {
            if z.block(op) && op.repeats() {
                z.t += d.t_extra as u32;
                z.pc = pc;
            }
        }
        Im(m) => z.im = m,
        LdIA => z.i = z.a,
        LdRA => z.r = z.a,
        LdAI => z.ld_a_i(),
        LdAR => z.ld_a_r(),
    }
}
