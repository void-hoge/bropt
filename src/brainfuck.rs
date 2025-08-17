use std::cmp;
use std::collections::{BTreeMap, HashSet};
use std::io::{self, Read, Write};
use std::iter::Peekable;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum InstType {
    ShiftInc,
    Output,
    Input,
    Seek,
    Skip,
    Set,
    Mulzero,
    Mul,
    Open,
    Close,
}

#[derive(Debug, Clone)]
pub struct Inst {
    cmd: InstType,
    inc: u8,
    delta: i16,
    arg: i32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BaseInst {
    Inc(u8),
    Shift(i32),
    Output,
    Input,
    Reset,
    Mul(i32, u8),
    Seek(i32),
    Skip(i32, u8, i16),
    Block(Vec<BaseInst>, bool),
}

pub fn parse(code: &str) -> Vec<BaseInst> {
    fn parse_block<I: Iterator<Item = char>>(
        iter: &mut I,
        in_block: bool,
    ) -> Result<(Vec<BaseInst>, bool), String> {
        let mut prog = Vec::new();
        let mut delta: i32 = 0;
        let mut stability = true;
        while let Some(ch) = iter.next() {
            match ch {
                '+' => prog.push(BaseInst::Inc(1)),
                '-' => prog.push(BaseInst::Inc(u8::MAX)),
                '>' => {
                    prog.push(BaseInst::Shift(1));
                    delta += 1;
                }
                '<' => {
                    prog.push(BaseInst::Shift(-1));
                    delta -= 1;
                }
                '.' => prog.push(BaseInst::Output),
                ',' => prog.push(BaseInst::Input),
                '[' => {
                    let (block, block_stability) = parse_block(iter, true)?;
                    stability &= block_stability;
                    prog.push(BaseInst::Block(block, block_stability));
                }
                ']' => {
                    return if in_block {
                        Ok((prog, stability && delta == 0))
                    } else {
                        Err("Unmatched ]".to_string())
                    };
                }
                _ => continue,
            }
        }
        if in_block {
            Err("Unmatched [".to_string())
        } else {
            Ok((prog, stability && delta == 0))
        }
    }
    let (block, _) = parse_block(&mut code.chars(), false).unwrap();
    block
}

pub fn compress(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    fn compress_block(block: Vec<BaseInst>) -> Vec<BaseInst> {
        let mut iter = block.into_iter().peekable();
        let mut compressed = Vec::with_capacity(iter.size_hint().0);
        while let Some(inst) = iter.next() {
            match inst {
                BaseInst::Inc(mut val) => {
                    while let Some(BaseInst::Inc(next)) = iter.peek() {
                        val += *next;
                        iter.next();
                    }
                    if val != 0 {
                        compressed.push(BaseInst::Inc(val));
                    }
                }
                BaseInst::Shift(mut off) => {
                    while let Some(BaseInst::Shift(next)) = iter.peek() {
                        off += *next;
                        iter.next();
                    }
                    if off != 0 {
                        compressed.push(BaseInst::Shift(off));
                    }
                }
                BaseInst::Block(inner, stability) => {
                    compressed.push(BaseInst::Block(compress_block(inner), stability));
                }
                other => compressed.push(other),
            }
        }
        compressed
    }
    compress_block(prog)
}

pub fn fold_simple_loops(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    fn gcd(mut a: u32, mut b: u32) -> u32 {
        while b != 0 {
            (a, b) = (b, a % b);
        }
        a
    }
    fn fold_block(block: Vec<BaseInst>) -> Vec<BaseInst> {
        block
            .into_iter()
            .map(|inst| match inst {
                BaseInst::Block(inner, stability) => {
                    let inner = fold_block(inner);
                    if inner.len() == 1 {
                        match inner[0] {
                            BaseInst::Inc(x) if gcd(x as u32, 256) == 1 => BaseInst::Reset,
                            BaseInst::Shift(n) => BaseInst::Seek(n),
                            _ => BaseInst::Block(inner, stability),
                        }
                    } else {
                        BaseInst::Block(inner, stability)
                    }
                }
                other => other,
            })
            .collect()
    }
    fold_block(prog)
}

pub fn fold_skip_loops(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    let mut folded = Vec::with_capacity(prog.len());
    for inst in prog {
        match inst {
            BaseInst::Block(inner, flag) => {
                let folded_inner = fold_skip_loops(inner);
                let mut ptr: i32 = 0;
                let mut inc_detected = false;
                let mut inc_amount: u8 = 0;
                let mut inc_offset: i32 = 0;
                let mut valid = true;
                for ins in &folded_inner {
                    match ins {
                        BaseInst::Shift(offset) => {
                            ptr += offset;
                        }
                        BaseInst::Inc(n) if !inc_detected => {
                            inc_detected = true;
                            inc_amount = *n;
                            inc_offset = ptr;
                        }
                        _ => {
                            valid = false;
                            break;
                        }
                    }
                }
                if valid && inc_detected && (i16::MIN as i32..i16::MAX as i32).contains(&inc_offset)
                {
                    folded.push(BaseInst::Skip(ptr, inc_amount, inc_offset as i16));
                } else {
                    folded.push(BaseInst::Block(folded_inner, flag));
                }
            }
            other => folded.push(other),
        }
    }
    folded
}

pub fn fold_mul_loops(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    let mut folded = Vec::with_capacity(prog.len());
    for inst in prog {
        match inst {
            BaseInst::Block(inner, stable) => {
                let folded_inner = fold_mul_loops(inner);
                if stable
                    && folded_inner
                        .iter()
                        .all(|ins| matches!(ins, BaseInst::Inc(..) | BaseInst::Shift(..)))
                {
                    let mut ptr: i32 = 0;
                    let mut changes: BTreeMap<i32, u8> = BTreeMap::new();
                    changes.insert(0, 0);
                    for inst in &folded_inner {
                        match inst {
                            BaseInst::Inc(val) => {
                                let entry = changes.entry(ptr).or_insert(0);
                                *entry += *val;
                            }
                            BaseInst::Shift(offset) => ptr += offset,
                            _ => unreachable!(),
                        }
                    }
                    if let Some(&u8::MAX) = changes.get(&0) {
                        let targets: Vec<(i32, u8)> = changes
                            .into_iter()
                            .filter(|&(offset, weight)| offset != 0 && weight != 0)
                            .collect();
                        for (offset, weight) in targets {
                            folded.push(BaseInst::Mul(offset, weight));
                        }
                        folded.push(BaseInst::Reset);
                        continue;
                    }
                }
                folded.push(BaseInst::Block(folded_inner, stable));
            }
            other => folded.push(other),
        }
    }
    folded
}

pub fn remove_dead_writes(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    fn remove_block(prog: Vec<BaseInst>, stable: bool) -> Vec<BaseInst> {
        if !stable {
            prog.into_iter()
                .map(|inst| match inst {
                    BaseInst::Block(inner, flag) => {
                        BaseInst::Block(remove_block(inner, flag), flag)
                    }
                    other => other,
                })
                .collect()
        } else {
            let mut targets = HashSet::<i32>::new();
            let mut ptr: i32 = 0;
            let mut removed = Vec::with_capacity(prog.len());
            for inst in prog.into_iter().rev() {
                match inst {
                    BaseInst::Shift(offset) => {
                        ptr -= offset;
                        removed.push(BaseInst::Shift(offset));
                    }
                    BaseInst::Reset => {
                        if targets.insert(ptr) {
                            removed.push(BaseInst::Reset);
                        }
                    }
                    BaseInst::Input => {
                        targets.insert(ptr);
                        removed.push(BaseInst::Input)
                    }
                    BaseInst::Output => {
                        targets.remove(&ptr);
                        removed.push(BaseInst::Output);
                    }
                    BaseInst::Mul(offset, weight) => {
                        let target = ptr + offset;
                        targets.remove(&ptr);
                        if !targets.contains(&target) {
                            removed.push(BaseInst::Mul(offset, weight));
                        }
                    }
                    BaseInst::Inc(n) => {
                        if !targets.contains(&ptr) {
                            removed.push(BaseInst::Inc(n));
                        }
                    }
                    BaseInst::Seek(offset) => {
                        targets.clear();
                        removed.push(BaseInst::Seek(offset));
                    }
                    BaseInst::Skip(offset, inc, delta) => {
                        targets.clear();
                        removed.push(BaseInst::Skip(offset, inc, delta));
                    }
                    BaseInst::Block(inner, flag) => {
                        targets.clear();
                        let removed_inner = remove_block(inner, flag);
                        removed.push(BaseInst::Block(removed_inner, flag));
                    }
                }
            }
            removed.reverse();
            removed
        }
    }
    remove_block(prog, false)
}

pub fn move_repeating_resets(prog: Vec<BaseInst>) -> Vec<BaseInst> {
    let mut moved = Vec::with_capacity(prog.len());
    for inst in prog {
        match inst {
            BaseInst::Block(block, flag) => {
                let moved_block = move_repeating_resets(block);
                if flag
                    && moved_block
                        .iter()
                        .all(|ins| !matches!(ins, BaseInst::Block(..)))
                {
                    let mut unremovable = HashSet::<i32>::new();
                    unremovable.insert(0);
                    let mut ptr: i32 = 0;
                    for ins in &moved_block {
                        match ins {
                            BaseInst::Shift(offset) => {
                                ptr += offset;
                            }
                            BaseInst::Output => {
                                unremovable.insert(ptr);
                            }
                            BaseInst::Mul(..) => {
                                unremovable.insert(ptr);
                            }
                            _ => {}
                        }
                    }
                    let mut seq = Vec::with_capacity(moved_block.len());
                    let mut removed = Vec::new();
                    ptr = 0;
                    for ins in moved_block.iter().rev() {
                        match ins {
                            BaseInst::Shift(offset) => {
                                ptr -= *offset;
                                seq.push(BaseInst::Shift(*offset));
                            }
                            BaseInst::Reset => {
                                if !unremovable.contains(&ptr) {
                                    removed.push(ptr);
                                } else {
                                    seq.push(BaseInst::Reset);
                                }
                            }
                            BaseInst::Inc(val) => {
                                unremovable.insert(ptr);
                                seq.push(BaseInst::Inc(*val));
                            }
                            BaseInst::Mul(offset, weight) => {
                                let target = ptr + *offset;
                                unremovable.insert(target);
                                seq.push(BaseInst::Mul(*offset, *weight));
                            }
                            BaseInst::Output => seq.push(BaseInst::Output),
                            BaseInst::Input => seq.push(BaseInst::Input),
                            BaseInst::Seek(..) | BaseInst::Skip(..) | BaseInst::Block(..) => {
                                unreachable!()
                            }
                        }
                    }
                    seq.reverse();
                    if removed.is_empty() {
                        moved.push(BaseInst::Block(seq, flag));
                    } else {
                        let mut moved_sets: Vec<BaseInst> = Vec::new();
                        for offset in removed {
                            moved_sets.push(BaseInst::Shift(offset));
                            moved_sets.push(BaseInst::Reset);
                            moved_sets.push(BaseInst::Shift(-offset));
                        }
                        seq = vec![BaseInst::Block(seq, flag)];
                        seq.extend(moved_sets);
                        moved.push(BaseInst::Block(seq, true));
                    }
                    continue;
                }
                moved.push(BaseInst::Block(moved_block, flag));
            }
            other => moved.push(other),
        }
    }
    moved
}

pub fn flatten(prog: Vec<BaseInst>) -> Vec<Inst> {
    fn pick_inc<I: Iterator<Item = BaseInst>>(iter: &mut Peekable<I>) -> u8 {
        if let Some(BaseInst::Inc(value)) = iter.peek() {
            let value = *value;
            iter.next();
            return value;
        }
        0
    }
    fn pick_shift<I: Iterator<Item = BaseInst>>(iter: &mut Peekable<I>) -> i16 {
        if let Some(BaseInst::Shift(delta)) = iter.peek() {
            let delta = *delta;
            if i16::MIN as i32 <= delta && delta <= i16::MAX as i32 {
                iter.next();
                return delta as i16;
            }
        }
        0
    }
    fn flatten_block<I: Iterator<Item = BaseInst>>(iter: &mut Peekable<I>) -> Vec<Inst> {
        let mut flat = Vec::new();
        while let Some(inst) = iter.next() {
            match inst {
                BaseInst::Inc(inc) => {
                    let delta = pick_shift(iter);
                    flat.push(Inst {
                        cmd: InstType::ShiftInc,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Shift(arg) => {
                    let arg = arg;
                    match iter.peek() {
                        Some(BaseInst::Reset) => {
                            iter.next();
                            let inc = pick_inc(iter);
                            let delta = pick_shift(iter);
                            flat.push(Inst {
                                cmd: InstType::Set,
                                arg: arg,
                                inc: inc,
                                delta: delta,
                            });
                        }
                        Some(BaseInst::Output) => {
                            iter.next();
                            let inc = pick_inc(iter);
                            let delta = pick_shift(iter);
                            flat.push(Inst {
                                cmd: InstType::Output,
                                arg: arg,
                                inc: inc,
                                delta: delta,
                            });
                        }
                        Some(BaseInst::Input) => {
                            iter.next();
                            let inc = pick_inc(iter);
                            let delta = pick_shift(iter);
                            flat.push(Inst {
                                cmd: InstType::Input,
                                arg: arg,
                                inc: inc,
                                delta: delta,
                            });
                        }
                        _ => {
                            let inc = pick_inc(iter);
                            let delta = pick_shift(iter);
                            flat.push(Inst {
                                cmd: InstType::ShiftInc,
                                arg: arg,
                                inc: inc,
                                delta: delta,
                            });
                        }
                    }
                }
                BaseInst::Output => {
                    let inc = pick_inc(iter);
                    let delta = pick_shift(iter);
                    flat.push(Inst {
                        cmd: InstType::Output,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Input => {
                    let inc = pick_inc(iter);
                    let delta = pick_shift(iter);
                    flat.push(Inst {
                        cmd: InstType::Input,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Reset => {
                    let inc = pick_inc(iter);
                    let delta = pick_shift(iter);
                    flat.push(Inst {
                        cmd: InstType::Set,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Mul(offset, weight) => {
                    if let Some(BaseInst::Reset) = iter.peek() {
                        iter.next();
                        let delta = pick_shift(iter);
                        flat.push(Inst {
                            cmd: InstType::Mulzero,
                            arg: offset,
                            inc: weight,
                            delta: delta,
                        });
                    } else {
                        flat.push(Inst {
                            cmd: InstType::Mul,
                            arg: offset,
                            inc: weight,
                            delta: 0,
                        });
                    }
                }
                BaseInst::Seek(offset) => {
                    let delta = pick_shift(iter);
                    let inc = pick_inc(iter);
                    flat.push(Inst {
                        cmd: InstType::Seek,
                        arg: offset,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Skip(offset, inc, delta) => {
                    flat.push(Inst {
                        cmd: InstType::Skip,
                        arg: offset,
                        inc: inc,
                        delta: delta,
                    });
                }
                BaseInst::Block(block, _) => {
                    let mut iter_block = block.into_iter().peekable();
                    let inc = pick_inc(&mut iter_block);
                    let delta = pick_shift(&mut iter_block);
                    let flat_block = flatten_block(&mut iter_block);
                    flat.push(Inst {
                        cmd: InstType::Open,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                    flat.extend(flat_block);
                    flat.push(Inst {
                        cmd: InstType::Close,
                        arg: 0,
                        inc: inc,
                        delta: delta,
                    });
                }
            }
        }
        flat
    }
    let mut iter = prog.into_iter().peekable();
    let mut flat = flatten_block(&mut iter);
    let mut stack = Vec::new();
    for idx in 0..flat.len() {
        match flat[idx].cmd {
            InstType::Open => {
                stack.push(idx);
            }
            InstType::Close => {
                let open = stack.pop().unwrap();
                flat[open].arg = idx as i32;
                flat[idx].arg = open as i32;
            }
            _ => {}
        }
    }
    flat
}

#[allow(dead_code)]
#[inline]
pub fn run<const FLUSH: bool>(prog: Vec<Inst>, length: usize) {
    let mut data = vec![0u8; length];
    let mut dp: usize = 0;
    let mut ip: usize = 0;
    while ip < prog.len() {
        let Inst {
            cmd,
            arg,
            inc,
            delta,
        } = &prog[ip];
        if *cmd == InstType::ShiftInc {
            dp = (dp as isize + *arg as isize) as usize;
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Output {
            dp = (dp as isize + *arg as isize) as usize;
            print!("{}", data[dp] as char);
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
            if FLUSH {
                io::stdout().flush().unwrap();
            }
        } else if *cmd == InstType::Input {
            dp = (dp as isize + *arg as isize) as usize;
            let mut buf = [0u8];
            if io::stdin().read_exact(&mut buf).is_ok() {
                data[dp] = buf[0];
            } else {
                data[dp] = 0u8;
            }
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Seek {
            while data[dp] != 0 {
                dp = (dp as isize + *arg as isize) as usize;
            }
            dp = (dp as isize + *delta as isize) as usize;
            data[dp] += *inc;
        } else if *cmd == InstType::Skip {
            while data[dp] != 0 {
                let pos = (dp as isize + *delta as isize) as usize;
                data[pos] += *inc;
                dp = (dp as isize + *arg as isize) as usize;
            }
        } else if *cmd == InstType::Set {
            dp = (dp as isize + *arg as isize) as usize;
            data[dp] = *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Mul {
            if data[dp] != 0 {
                let pos = (dp as isize + *arg as isize) as usize;
                data[pos] += data[dp] * *inc;
            }
        } else if *cmd == InstType::Mulzero {
            if data[dp] != 0 {
                let pos = (dp as isize + *arg as isize) as usize;
                data[pos] += data[dp] * *inc;
                data[dp] = 0;
            }
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Open {
            if data[dp] == 0 {
                ip = *arg as usize;
            } else {
                data[dp] += *inc;
                dp = (dp as isize + *delta as isize) as usize;
            }
        } else
        /* if *cmd == InstType::Close */
        {
            if data[dp] != 0 {
                ip = *arg as usize;
                data[dp] += *inc;
                dp = (dp as isize + *delta as isize) as usize;
            }
        }
        ip += 1;
    }
}

#[allow(dead_code)]
#[inline]
pub fn run_with_state(prog: Vec<Inst>, length: usize, input: &[u8]) -> (Vec<u8>, Vec<u8>, usize) {
    let mut data = vec![0u8; length];
    let mut dp: usize = 0;
    let mut ip: usize = 0;
    let mut output = Vec::new();
    let mut in_idx = 0usize;
    while ip < prog.len() {
        let Inst {
            cmd,
            arg,
            inc,
            delta,
        } = &prog[ip];
        if *cmd == InstType::ShiftInc {
            dp = (dp as isize + *arg as isize) as usize;
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Output {
            dp = (dp as isize + *arg as isize) as usize;
            output.push(data[dp]);
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Input {
            dp = (dp as isize + *arg as isize) as usize;
            if in_idx < input.len() {
                data[dp] = input[in_idx];
                in_idx += 1;
            } else {
                data[dp] = 0u8;
            }
            data[dp] += *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Seek {
            while data[dp] != 0 {
                dp = (dp as isize + *arg as isize) as usize;
            }
            dp = (dp as isize + *delta as isize) as usize;
            data[dp] += *inc;
        } else if *cmd == InstType::Skip {
            while data[dp] != 0 {
                let pos = (dp as isize + *delta as isize) as usize;
                data[pos] += *inc;
                dp = (dp as isize + *arg as isize) as usize;
            }
        } else if *cmd == InstType::Set {
            dp = (dp as isize + *arg as isize) as usize;
            data[dp] = *inc;
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Mul {
            if data[dp] != 0 {
                let pos = (dp as isize + *arg as isize) as usize;
                data[pos] += data[dp] * *inc;
            }
        } else if *cmd == InstType::Mulzero {
            if data[dp] != 0 {
                let pos = (dp as isize + *arg as isize) as usize;
                data[pos] += data[dp] * *inc;
                data[dp] = 0;
            }
            dp = (dp as isize + *delta as isize) as usize;
        } else if *cmd == InstType::Open {
            if data[dp] == 0 {
                ip = *arg as usize;
            } else {
                data[dp] += *inc;
                dp = (dp as isize + *delta as isize) as usize;
            }
        } else
        /* if *cmd == InstType::Close */
        {
            if data[dp] != 0 {
                ip = *arg as usize;
                data[dp] += *inc;
                dp = (dp as isize + *delta as isize) as usize;
            }
        }
        ip += 1;
    }
    (output, data, dp)
}

#[allow(dead_code)]
#[inline]
pub fn unsafe_run<const FLUSH: bool>(prog: Vec<Inst>, length: usize, offset: isize) {
    let mut ip = 0usize;
    let mut data = vec![0u8; length];
    unsafe {
        let mut ptr = data.as_mut_ptr().offset(offset);
        while ip < prog.len() {
            let Inst {
                cmd,
                arg,
                inc,
                delta,
            } = &prog[ip];
            if *cmd == InstType::Output {
                ptr = ptr.offset(*arg as isize);
                print!("{}", ptr.read() as char);
                ptr.write(ptr.read() + *inc);
                ptr = ptr.offset(*delta as isize);
                if FLUSH {
                    io::stdout().flush().unwrap();
                }
            } else if *cmd == InstType::Input {
                ptr = ptr.offset(*arg as isize);
                let mut buf = [0u8];
                if io::stdin().read_exact(&mut buf).is_ok() {
                    ptr.write(buf[0]);
                } else {
                    ptr.write(0);
                }
                ptr.write(ptr.read() + *inc);
                ptr = ptr.offset(*delta as isize);
            } else if *cmd == InstType::ShiftInc {
                ptr = ptr.offset(*arg as isize);
                ptr.write(ptr.read() + *inc);
                ptr = ptr.offset(*delta as isize);
            } else if *cmd == InstType::Seek {
                while ptr.read() != 0 {
                    ptr = ptr.offset(*arg as isize);
                }
                ptr = ptr.offset(*delta as isize);
                ptr.write(ptr.read() + *inc);
            } else if *cmd == InstType::Skip {
                while ptr.read() != 0 {
                    let pos = ptr.offset(*delta as isize);
                    pos.write(pos.read() + *inc);
                    ptr = ptr.offset(*arg as isize);
                }
            } else if *cmd == InstType::Set {
                ptr = ptr.offset(*arg as isize);
                ptr.write(*inc);
                ptr = ptr.offset(*delta as isize);
            } else if *cmd == InstType::Mulzero {
                let pos = ptr.offset(*arg as isize);
                pos.write(pos.read() + ptr.read() * *inc);
                ptr.write(0);
                ptr = ptr.offset(*delta as isize);
            } else if *cmd == InstType::Mul {
                let pos = ptr.offset(*arg as isize);
                pos.write(pos.read() + ptr.read() * *inc);
            } else if *cmd == InstType::Open {
                if ptr.read() == 0 {
                    ip = *arg as usize;
                } else {
                    ptr.write(ptr.read() + *inc);
                    ptr = ptr.offset(*delta as isize);
                }
            } else
            /* if *cmd == InstType::Close */
            {
                if ptr.read() != 0 {
                    ip = *arg as usize;
                    ptr.write(ptr.read() + *inc);
                    ptr = ptr.offset(*delta as isize);
                }
            }
            ip += 1;
        }
    }
}

pub fn compile(code: &str) -> Vec<Inst> {
    let mut prog = parse(&code);
    prog = compress(prog);
    prog = fold_simple_loops(prog);
    prog = fold_mul_loops(prog);
    prog = remove_dead_writes(prog);
    prog = remove_dead_writes(prog);
    prog = move_repeating_resets(prog);
    prog = compress(prog);
    prog = fold_simple_loops(prog);
    prog = fold_mul_loops(prog);
    prog = remove_dead_writes(prog);
    prog = remove_dead_writes(prog);
    prog = move_repeating_resets(prog);
    prog = compress(prog);
    prog = fold_simple_loops(prog);
    prog = fold_mul_loops(prog);
    prog = fold_skip_loops(prog);
    flatten(prog)
}

pub fn get_offset(prog: &Vec<Inst>) -> isize {
    let mut offset = 0isize;
    for inst in prog {
        match inst.cmd {
            InstType::Mul => {
                offset = cmp::max(offset, -inst.arg as isize);
            }
            InstType::Mulzero => {
                offset = cmp::max(offset, -inst.arg as isize);
            }
            _ => {}
        }
    }
    offset
}
