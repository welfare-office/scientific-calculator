//! Long-running computation jobs (huge factorials, huge integer powers).
//!
//! Design:
//! - work proceeds in BLOCK-sized chunks; each chunk product is computed by a
//!   balanced product tree whose leaf stage is SIMD-accelerated on wasm32
//! - finished chunk products merge into a `pending` list via binary-counter
//!   rules (equal levels combine), so total multiplication stays near-balanced
//! - `checkpoint()` / `restore()` serialise {cursor, pending} so a worker can
//!   persist progress to OPFS and resume after reload/cancel

use malachite::num::arithmetic::traits::Pow;
use malachite::num::basic::traits::One;
use malachite::platform::Limb;
use malachite::Natural;
use std::convert::TryFrom;

const MAGIC: u32 = 0x434C4A01; // 'CLJ' v1
const BLOCK: u64 = 16_384;

/// Maximum factorial operand accepted for the async path (1e6! = ~5.5M digits).
pub const LONG_FACT_MAX: u64 = 1_000_000;
/// Above this, factorial is computed synchronously into a bigint.
pub const BIGINT_FACT_MAX: u64 = 170;
/// Max decimal digits for an integer power handled at all.
pub const LONG_POW_MAX_DIGITS: f64 = 8_000_000.0;
/// Powers with more digits than this go async.
pub const POW_SYNC_MAX_DIGITS: f64 = 2_000.0;

#[derive(Clone, Debug, PartialEq)]
pub enum JobKind {
    /// n!
    Fact(u64),
    /// base^exp, integer exponent
    Pow { base: u64, exp: u64 },
}

struct Pending {
    level: u32,
    val: Natural,
}

pub struct LongJob {
    kind: JobKind,
    cursor: u64, // next unit of work (multiplier index / consumed exponent)
    pending: Vec<Pending>,
}

fn total_units(kind: &JobKind) -> u64 {
    match kind {
        JobKind::Fact(n) => *n,
        JobKind::Pow { exp, .. } => *exp,
    }
}

impl LongJob {
    pub fn new_factorial(n: u64) -> Self {
        LongJob {
            kind: JobKind::Fact(n),
            cursor: 1,
            pending: Vec::new(),
        }
    }

    pub fn new_pow(base: u64, exp: u64) -> Self {
        LongJob {
            kind: JobKind::Pow { base, exp },
            cursor: 0,
            pending: Vec::new(),
        }
    }

    pub fn key(&self) -> String {
        match self.kind {
            JobKind::Fact(n) => format!("fact:{n}"),
            JobKind::Pow { base, exp } => format!("pow:{base}:{exp}"),
        }
    }

    pub fn label(&self) -> String {
        match self.kind {
            JobKind::Fact(n) => format!("{n}!"),
            JobKind::Pow { base, exp } => format!("{base}^{exp}"),
        }
    }

    pub fn total(&self) -> u64 {
        total_units(&self.kind)
    }

    pub fn done_units(&self) -> u64 {
        match self.kind {
            JobKind::Fact(n) => (self.cursor.saturating_sub(1)).min(n),
            JobKind::Pow { exp, .. } => self.cursor.min(exp),
        }
    }

    pub fn progress(&self) -> f64 {
        self.done_units() as f64 / self.total() as f64
    }

    pub fn done(&self) -> bool {
        self.done_units() >= self.total()
    }

    /// One chunk of work. Returns progress in [0,1].
    pub fn step(&mut self) -> f64 {
        if self.done() {
            return 1.0;
        }
        match self.kind {
            JobKind::Fact(n) => {
                let end = (self.cursor + BLOCK).min(n + 1);
                let block = range_product(self.cursor, end);
                self.push_pending(block);
                self.cursor = end;
            }
            JobKind::Pow { base, exp } => {
                let take = BLOCK.min(exp - self.cursor);
                let term = Natural::from(base).pow(take);
                self.push_pending(term);
                self.cursor += take;
            }
        }
        self.progress()
    }

    /// merge a finished block into pending, combining equal levels
    fn push_pending(&mut self, val: Natural) {
        let mut cur = Pending { level: 0, val };
        while let Some(top) = self.pending.last() {
            if top.level != cur.level {
                break;
            }
            let top = self.pending.pop().unwrap();
            cur = Pending {
                level: cur.level + 1,
                val: top.val * cur.val,
            };
        }
        self.pending.push(cur);
    }

    /// Final result (call when done()).
    pub fn result(&self) -> Natural {
        let mut acc = Natural::ONE;
        for p in &self.pending {
            acc *= &p.val;
        }
        acc
    }

    // ---- checkpoint serialisation ---------------------------------------

    pub fn checkpoint(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&MAGIC.to_le_bytes());
        match self.kind {
            JobKind::Fact(n) => {
                out.push(0u8);
                out.extend_from_slice(&n.to_le_bytes());
            }
            JobKind::Pow { base, exp } => {
                out.push(1u8);
                out.extend_from_slice(&base.to_le_bytes());
                out.extend_from_slice(&exp.to_le_bytes());
            }
        }
        out.extend_from_slice(&self.cursor.to_le_bytes());
        out.extend_from_slice(&(self.pending.len() as u32).to_le_bytes());
        for p in &self.pending {
            let limbs = p.val.to_limbs_asc();
            out.extend_from_slice(&p.level.to_le_bytes());
            out.extend_from_slice(&(limbs.len() as u64).to_le_bytes());
            for limb in limbs {
                let v = u64::from(limb);
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    pub fn restore(bytes: &[u8]) -> Option<LongJob> {
        let mut r = Reader { b: bytes, i: 0 };
        if r.u32()? != MAGIC {
            return None;
        }
        let kind = match r.u8()? {
            0 => JobKind::Fact(r.u64()?),
            1 => JobKind::Pow {
                base: r.u64()?,
                exp: r.u64()?,
            },
            _ => return None,
        };
        let cursor = r.u64()?;
        let count = r.u32()?;
        let mut pending = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let level = r.u32()?;
            let n = r.u64()? as usize;
            let mut limbs: Vec<Limb> = Vec::with_capacity(n);
            for _ in 0..n {
                limbs.push(Limb::try_from(r.u64()?).ok()?);
            }
            pending.push(Pending {
                level,
                val: Natural::from_limbs_asc(&limbs),
            });
        }
        Some(LongJob {
            kind,
            cursor,
            pending,
        })
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.i..self.i + n)?;
        self.i += n;
        Some(s)
    }
    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|s| s[0])
    }
    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
    }
    fn u64(&mut self) -> Option<u64> {
        self.take(8)
            .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
    }
}

// ---------------------------------------------------------------------------
// Chunk product: SIMD-accelerated leaf products + balanced product tree
// ---------------------------------------------------------------------------

/// product of the half-open range [start, end)
fn range_product(start: u64, end: u64) -> Natural {
    if start >= end {
        return Natural::ONE;
    }
    // leaf stage: pair products via SIMD where the product fits u64.
    // pairs (k, k+1) fit u64 while k <= 4_294_967_294; our job domain is far
    // below that, but guard anyway and fall back to single leaves.
    let mut leaves: Vec<u64> = Vec::new();
    if end - 1 <= 4_294_967_294 {
        leaf_pairs(start, end, &mut leaves);
    } else {
        for k in start..end {
            leaves.push(k); // single; cannot overflow since leaf IS k
        }
    }
    // fold u64 leaves -> u128 (pair of pairs fits: leaf ~ (1e12)^2 = 1e24 << 2^128
    // when leaf values are small; guard: combine only if both < 2^64/2^32)
    let mut level: Vec<Natural> = Vec::with_capacity(leaves.len() / 2 + 1);
    let mut i = 0;
    while i + 1 < leaves.len() {
        let (a, b) = (leaves[i], leaves[i + 1]);
        if let Some(prod) = a.checked_mul(b) {
            level.push(Natural::from(prod));
        } else {
            level.push(Natural::from(a) * Natural::from(b));
        }
        i += 2;
    }
    if i < leaves.len() {
        level.push(Natural::from(leaves[i]));
    }
    // balanced tree of Natural products
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len() / 2 + 1);
        let mut it = level.into_iter();
        while let Some(a) = it.next() {
            next.push(match it.next() {
                Some(b) => a * b,
                None => a,
            });
        }
        level = next;
    }
    level.pop().unwrap_or(Natural::ONE)
}

/// push pair-products k*(k+1) over [start,end) into `leaves`.
/// wasm32 simd128 path multiplies two pairs per instruction.
fn leaf_pairs(mut k: u64, end: u64, leaves: &mut Vec<u64>) {
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        use core::arch::wasm32::*;
        // four terms per iteration: [k,k+2] * [k+1,k+3]
        while k + 3 < end {
            let a = u64x2(k, k + 2);
            let b = u64x2(k + 1, k + 3);
            let p = u64x2_mul(a, b);
            leaves.push(u64x2_extract_lane::<0>(p));
            leaves.push(u64x2_extract_lane::<1>(p));
            k += 4;
        }
    }
    while k < end {
        if k + 1 < end {
            leaves.push(k * (k + 1));
            k += 2;
        } else {
            leaves.push(k);
            k += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// Synchronous bigint helpers (results too big for f64 but small enough to
// compute inline)
// ---------------------------------------------------------------------------

pub fn factorial_big(n: u64) -> Natural {
    let mut job = LongJob::new_factorial(n);
    while !job.done() {
        job.step();
    }
    job.result()
}

pub fn pow_big(base: u64, exp: u64) -> Natural {
    let mut job = LongJob::new_pow(base, exp);
    while !job.done() {
        job.step();
    }
    job.result()
}

/// "2.824229e456573"-style short form from a full decimal string.
pub fn approx_of(decimal: &str) -> String {
    let digits = decimal.len();
    if digits <= 12 {
        return decimal.to_string();
    }
    let mantissa = &decimal[..8.min(digits)];
    format!("{}.{}e{}", &mantissa[..1], &mantissa[1..], digits - 1)
}

