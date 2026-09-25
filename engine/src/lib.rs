use wasm_bindgen::prelude::*;

pub mod long;

/// Async computation job exposed to JS: the worker drives `step()` in a loop,
/// persists `checkpoint()` bytes to OPFS, and can `restore()` to resume.
#[wasm_bindgen]
pub struct WasmJob {
    inner: long::LongJob,
}

#[wasm_bindgen]
impl WasmJob {
    #[wasm_bindgen(js_name = "newFactorial")]
    pub fn new_factorial(n: f64) -> WasmJob {
        WasmJob {
            inner: long::LongJob::new_factorial(n as u64),
        }
    }

    #[wasm_bindgen(js_name = "newPow")]
    pub fn new_pow(base: f64, exp: f64) -> WasmJob {
        WasmJob {
            inner: long::LongJob::new_pow(base as u64, exp as u64),
        }
    }

    /// Restore from checkpoint bytes; returns undefined if the bytes are invalid.
    #[wasm_bindgen(js_name = "restore")]
    pub fn restore_js(bytes: &[u8]) -> Option<WasmJob> {
        long::LongJob::restore(bytes).map(|inner| WasmJob { inner })
    }

    /// Parse a job spec "fact:100000" / "pow:2:999999"; undefined if unknown.
    #[wasm_bindgen(js_name = "fromSpec")]
    pub fn from_spec(spec: &str) -> Option<WasmJob> {
        let parts: Vec<&str> = spec.split(':').collect();
        let inner = match parts.as_slice() {
            ["fact", n] => n.parse::<u64>().ok().map(long::LongJob::new_factorial),
            ["pow", b, e] => match (b.parse::<u64>(), e.parse::<u64>()) {
                (Ok(b), Ok(e)) => Some(long::LongJob::new_pow(b, e)),
                _ => None,
            },
            _ => None,
        }?;
        Some(WasmJob { inner })
    }

    /// One chunk of work. Returns progress in [0,1]; 1.0 means finished.
    pub fn step(&mut self) -> f64 {
        self.inner.step()
    }

    pub fn checkpoint(&self) -> Vec<u8> {
        self.inner.checkpoint()
    }

    pub fn key(&self) -> String {
        self.inner.key()
    }

    pub fn label(&self) -> String {
        self.inner.label()
    }

    pub fn progress(&self) -> f64 {
        self.inner.progress()
    }

    #[wasm_bindgen(js_name = "isDone")]
    pub fn is_done(&self) -> bool {
        self.inner.done()
    }

    /// Full decimal expansion of the result. Expensive - call once when done.
    #[wasm_bindgen(js_name = "toDecimal")]
    pub fn to_decimal(&self) -> String {
        self.inner.result().to_string()
    }
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Op(char),          // '+' '-' '*' '/' '^' 'r' (y-th root) 'E' (EE: *10^)
    LParen,
    RParen,
    Func(&'static str),
    Fact,              // postfix !
}

// ---------------------------------------------------------------------------
// Recursive-descent evaluator
// ---------------------------------------------------------------------------

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    deg: bool,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    // expr := term (('+'|'-') term)*
    fn expr(&mut self) -> Result<f64, ()> {
        let mut v = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek() {
            let c = *c;
            self.next();
            let rhs = self.term()?;
            v = if c == '+' { v + rhs } else { v - rhs };
        }
        Ok(v)
    }

    // term := factor (('*'|'/'|implicit) factor)*
    fn term(&mut self) -> Result<f64, ()> {
        let mut v = self.factor()?;
        loop {
            match self.peek() {
                Some(Tok::Op('*')) => {
                    self.next();
                    v *= self.factor()?;
                }
                Some(Tok::Op('/')) => {
                    self.next();
                    v /= self.factor()?;
                }
                // implicit multiplication: 2π, 3(, )(, )sin(, )π
                Some(Tok::LParen) | Some(Tok::Func(_)) | Some(Tok::Num(_)) => {
                    v *= self.factor()?;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    // factor := unary ('^'|'r'|'E' factor)?   (right assoc)
    fn factor(&mut self) -> Result<f64, ()> {
        let base = self.unary()?;
        match self.peek() {
            Some(Tok::Op('^')) => {
                self.next();
                Ok(base.powf(self.factor()?))
            }
            Some(Tok::Op('r')) => {
                self.next();
                let y = self.factor()?;
                Ok(y.powf(1.0 / base))
            }
            Some(Tok::Op('E')) => {
                self.next();
                let e = self.factor()?;
                Ok(base * 10f64.powf(e))
            }
            _ => Ok(base),
        }
    }

    // unary := '-' unary | '+' unary | postfix
    fn unary(&mut self) -> Result<f64, ()> {
        match self.peek() {
            Some(Tok::Op('-')) => {
                self.next();
                Ok(-self.unary()?)
            }
            Some(Tok::Op('+')) => {
                self.next();
                self.unary()
            }
            _ => self.postfix(),
        }
    }

    // postfix := atom ('!'|'%')*
    fn postfix(&mut self) -> Result<f64, ()> {
        let mut v = self.atom()?;
        loop {
            match self.peek() {
                Some(Tok::Fact) => {
                    self.next();
                    v = factorial(v)?;
                }
                _ => break,
            }
        }
        Ok(v)
    }

    fn atom(&mut self) -> Result<f64, ()> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(n),
            Some(Tok::LParen) => {
                let v = self.expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(v),
                    _ => Err(()),
                }
            }
            Some(Tok::Func(name)) => {
                // argument: parenthesised expr, or next unary (sin 30)
                let arg = if matches!(self.peek(), Some(Tok::LParen)) {
                    self.next();
                    let a = self.expr()?;
                    match self.next() {
                        Some(Tok::RParen) => a,
                        _ => return Err(()),
                    }
                } else {
                    self.unary()?
                };
                Ok(apply_func(&name, arg, self.deg))
            }
            _ => Err(()),
        }
    }
}

fn apply_func(name: &str, x: f64, deg: bool) -> f64 {
    let to_rad = |v: f64| if deg { v.to_radians() } else { v };
    let to_deg = |v: f64| if deg { v.to_degrees() } else { v };
    match name {
        "sin" => to_rad(x).sin(),
        "cos" => to_rad(x).cos(),
        "tan" => to_rad(x).tan(),
        "asin" => to_deg(x.asin()),
        "acos" => to_deg(x.acos()),
        "atan" => to_deg(x.atan()),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "asinh" => x.asinh(),
        "acosh" => x.acosh(),
        "atanh" => x.atanh(),
        "ln" => x.ln(),
        "log2" => x.log2(),
        "log10" => x.log10(),
        "sqrt" => x.sqrt(),
        "cbrt" => x.cbrt(),
        "exp" => x.exp(),
        "exp10" => 10f64.powf(x),
        "exp2" => 2f64.powf(x),
        "inv" => 1.0 / x,
        "sq" => x * x,
        "cube" => x * x * x,
        "abs" => x.abs(),
        "floor" => x.floor(),
        "ceil" => x.ceil(),
        _ => f64::NAN,
    }
}

fn factorial(x: f64) -> Result<f64, ()> {
    if x < 0.0 || x.fract() != 0.0 || x > 170.0 {
        return Err(());
    }
    Ok((1..=x as u64).fold(1f64, |a, b| a * b as f64))
}

fn evaluate(toks: &[Tok], deg: bool) -> Result<f64, ()> {
    let mut p = Parser { toks, pos: 0, deg };
    let v = p.expr()?;
    if p.pos == toks.len() && v.is_finite() {
        Ok(v)
    } else {
        Err(())
    }
}

// ---------------------------------------------------------------------------
// Apple-style number formatting
// ---------------------------------------------------------------------------

fn format_value(v: f64) -> String {
    if !v.is_finite() {
        return "Error".into();
    }
    if v == 0.0 {
        return "0".into();
    }
    let abs = v.abs();
    if abs >= 1e16 || abs < 1e-8 {
        // scientific notation: 1.234568e+17
        let s = format!("{:.8e}", v); // 9 sig digits
        let mut parts = s.split('e');
        let mant = parts.next().unwrap().trim_end_matches('0').trim_end_matches('.');
        let exp: i32 = parts.next().unwrap().parse().unwrap_or(0);
        return format!("{}e{}", mant, exp);
    }
    // cap at 9 significant digits
    let digits = abs.log10().floor() as i32 + 1;
    let decimals = (9 - digits).max(0) as usize;
    let mut s = format!("{:.*}", decimals, v);
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    // thousands separators on integer part
    let neg = s.starts_with('-');
    let body = if neg { &s[1..] } else { &s };
    let (int_part, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let mut grouped = String::new();
    for (i, c) in int_part.chars().enumerate() {
        if i > 0 && (int_part.len() - i) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(c);
    }
    let mut out = if neg { format!("-{}", grouped) } else { grouped };
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    out
}

// ---------------------------------------------------------------------------
// Calculator state machine (expression model)
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub struct Calculator {
    toks: Vec<Tok>,
    entry: String,        // current digits being typed, "" when not typing
    just_eval: bool,      // display holds a result; next digit starts fresh
    deg: bool,
    second: bool,
    memory: f64,
    has_memory: bool,
    last_repeat: Option<Vec<Tok>>, // for repeated '='
    error: bool,
    pending_job: Option<String>, // "fact:100000" / "pow:2:999999" awaiting worker pickup
    message: Option<String>,     // transient readout text ("Too large")
    big_result: Option<(String, String)>, // (approx display, tape label)
    full_result: Option<String>,          // full decimal text for copy (sync path)
}

#[wasm_bindgen]
impl Calculator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Calculator {
        Calculator {
            toks: Vec::new(),
            entry: String::new(),
            just_eval: false,
            deg: true,
            second: false,
            memory: 0.0,
            has_memory: false,
            last_repeat: None,
            error: false,
            pending_job: None,
            message: None,
            big_result: None,
            full_result: None,
        }
    }

    // ---- queries -------------------------------------------------------

    pub fn display(&self) -> String {
        if self.error {
            return "Error".into();
        }
        if let Some(m) = &self.message {
            return m.clone();
        }
        if let Some((approx, _)) = &self.big_result {
            return approx.clone();
        }
        if !self.entry.is_empty() {
            return format_entry(&self.entry);
        }
        // show evaluation of committed expression so far
        match self.eval_current() {
            Ok(v) => format_value(v),
            Err(()) => "0".into(),
        }
    }

    /// expression preview shown above the main display
    pub fn tape(&self) -> String {
        if let Some((_, label)) = &self.big_result {
            return label.clone();
        }
        let mut s = String::new();
        for t in &self.toks {
            s.push_str(&tok_label(t));
        }
        if !self.entry.is_empty() {
            s.push_str(&self.entry);
        }
        s
    }

    pub fn is_deg(&self) -> bool {
        self.deg
    }
    pub fn is_second(&self) -> bool {
        self.second
    }
    pub fn is_error(&self) -> bool {
        self.error
    }
    pub fn has_entry(&self) -> bool {
        !self.entry.is_empty()
    }
    pub fn has_memory(&self) -> bool {
        self.has_memory
    }
    /// name of the pending binary operator for highlight, "" if none
    pub fn active_op(&self) -> String {
        if self.entry.is_empty() {
            if let Some(Tok::Op(c)) = self.toks.last() {
                return c.to_string();
            }
        }
        String::new()
    }

    /// drains a queued long-computation request; "" when none
    pub fn take_long_job(&mut self) -> String {
        self.pending_job.take().unwrap_or_default()
    }

    /// called by the UI when a worker finishes: show approx + tape label
    pub fn set_big_result(&mut self, approx: &str, label: &str) {
        self.big_result = Some((approx.to_string(), label.to_string()));
        self.toks.clear();
        self.entry.clear();
        self.just_eval = false;
        self.error = false;
    }

    pub fn has_big_result(&self) -> bool {
        self.big_result.is_some()
    }

    /// full decimal text when computed synchronously (async path streams it
    /// through the worker instead); "" when unavailable
    pub fn full_result(&self) -> String {
        self.full_result.clone().unwrap_or_default()
    }

    // ---- key input ------------------------------------------------------

    /// single entry point: press(id) where id is a key identifier
    pub fn press(&mut self, key: &str) {
        self.message = None;
        // a committed big result is consumed (or dismissed) by the next input
        if self.big_result.is_some() && !matches!(key, "2nd" | "deg" | "rad" | "toggle_deg") {
            let (approx, _) = self.big_result.take().unwrap();
            let seed = approx.parse::<f64>().ok().filter(|v| v.is_finite());
            self.toks.clear();
            self.entry.clear();
            self.just_eval = false;
            match key {
                "eq" | "mc" | "mr" => return,
                "ac" | "c" | "back" => {
                    self.all_clear();
                    return;
                }
                "m+" | "m-" => {
                    if let Some(v) = seed {
                        let sign = if key == "m+" { 1.0 } else { -1.0 };
                        self.memory += sign * v;
                        self.has_memory = true;
                    }
                    return;
                }
                "add" | "sub" | "mul" | "div" | "pow" | "root" => match seed {
                    Some(v) => self.toks.push(Tok::Num(v)),
                    None => return,
                },
                "neg" => match seed {
                    Some(v) => {
                        self.toks.push(Tok::Num(-v));
                        return;
                    }
                    None => return,
                },
                k if FUNC_NAMES.contains(&k) || k == "fact" || k == "pct" => match seed {
                    Some(v) => self.toks.push(Tok::Num(v)),
                    None => return,
                },
                _ => {} // digits, dot, parens, constants: fresh input
            }
        }
        match key {
            "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => {
                self.digit(key.chars().next().unwrap())
            }
            "dot" => self.dot(),
            "add" => self.binary('+'),
            "sub" => self.binary('-'),
            "mul" => self.binary('*'),
            "div" => self.binary('/'),
            "pow" => self.binary('^'),
            "root" => self.binary('r'),
            "eq" => self.equals(),
            "ac" => self.all_clear(),
            "c" => self.clear_entry(),
            "back" => self.backspace(),
            "neg" => self.negate(),
            "pct" => self.percent(),
            "lparen" => self.lparen(),
            "rparen" => self.rparen(),
            "fact" => self.fact(),
            "pi" => self.constant(std::f64::consts::PI),
            "e" => self.constant(std::f64::consts::E),
            "rand" => self.rand(),
            "ee" => self.ee(),
            "deg" => self.deg = true,
            "rad" => self.deg = false,
            "toggle_deg" => self.deg = !self.deg,
            "2nd" => self.second = !self.second,
            "mc" => {
                self.memory = 0.0;
                self.has_memory = false;
            }
            "mr" => {
                if self.has_memory {
                    self.constant(self.memory);
                }
            }
            "m+" => self.mem_add(1.0),
            "m-" => self.mem_add(-1.0),
            f if FUNC_NAMES.contains(&f) => {
                let name = FUNC_NAMES.iter().copied().find(|s| *s == f).unwrap();
                self.func(name)
            }
            _ => {}
        }
    }

    // ---- internals -------------------------------------------------------

    fn current_value(&self) -> f64 {
        self.eval_current().unwrap_or(0.0)
    }

    /// evaluate toks + entry-as-number, tolerating trailing operators
    fn eval_current(&self) -> Result<f64, ()> {
        let mut t = self.toks.clone();
        if !self.entry.is_empty() {
            t.push(Tok::Num(self.entry.parse().map_err(|_| ())?));
        }
        // drop trailing binary op / lparen / func for partial eval
        while matches!(
            t.last(),
            Some(Tok::Op(_)) | Some(Tok::LParen) | Some(Tok::Func(_))
        ) {
            t.pop();
        }
        // balance unclosed parens
        let mut depth = 0i32;
        for tok in &t {
            match tok {
                Tok::LParen => depth += 1,
                Tok::RParen => depth -= 1,
                _ => {}
            }
        }
        for _ in 0..depth.max(0) {
            t.push(Tok::RParen);
        }
        if t.is_empty() {
            return Ok(0.0);
        }
        evaluate(&t, self.deg)
    }

    fn commit_entry(&mut self) {
        if !self.entry.is_empty() {
            if let Ok(n) = self.entry.parse::<f64>() {
                self.toks.push(Tok::Num(n));
            }
            self.entry.clear();
        }
    }

    fn digit(&mut self, d: char) {
        if self.error || self.just_eval {
            self.toks.clear();
            self.just_eval = false;
            self.error = false;
        }
        // after ')' or postfix or constant → implicit multiply? Apple starts fresh number
        if self.entry == "0" {
            self.entry.clear();
        }
        if self.entry.len() < 16 {
            self.entry.push(d);
        }
    }

    fn dot(&mut self) {
        if self.error || self.just_eval {
            self.toks.clear();
            self.entry = "0".into();
            self.just_eval = false;
            self.error = false;
        }
        if self.entry.is_empty() {
            self.entry = "0".into();
        }
        if !self.entry.contains('.') && !self.entry.contains('e') {
            self.entry.push('.');
        }
    }

    fn ee(&mut self) {
        if self.entry.is_empty() || self.just_eval {
            self.entry = "1".into();
            self.just_eval = false;
        }
        if !self.entry.contains('e') {
            self.entry.push('e');
        }
    }

    fn binary(&mut self, op: char) {
        if self.error {
            return;
        }
        self.commit_entry();
        self.just_eval = false;
        match self.toks.last() {
            Some(Tok::Op(_)) => {
                *self.toks.last_mut().unwrap() = Tok::Op(op);
            }
            Some(Tok::LParen) | Some(Tok::Func(_)) | None => {
                // operator before any operand → operate on 0 or result
                if self.toks.is_empty() {
                    self.toks.push(Tok::Num(0.0));
                }
                self.toks.push(Tok::Op(op));
            }
            _ => self.toks.push(Tok::Op(op)),
        }
        self.last_repeat = None;
    }

    fn equals(&mut self) {
        if self.error {
            return;
        }
        // repeated '=' re-applies last binary op+operand
        if self.just_eval {
            if let Some(rep) = self.last_repeat.clone() {
                let mut t = self.toks.clone();
                t.extend(rep.iter().cloned());
                if let Ok(v) = evaluate(&t, self.deg) {
                    self.toks = vec![Tok::Num(v)];
                    return;
                }
            }
            return;
        }
        // capture last op + operand for repeat before eval
        self.commit_entry();
        // big integer power: detect [b ^ e] and route by size
        if let [Tok::Num(b), Tok::Op('^'), Tok::Num(e)] = self.toks.as_slice() {
            let (b, e) = (*b, *e);
            if e >= 0.0 && e.fract() == 0.0 && b.fract() == 0.0 && b.abs() <= 9e18 && e <= 1e15 {
                let digits = if b.abs() <= 1.0 { 1.0 } else { e * b.abs().log10() };
                if digits > long::LONG_POW_MAX_DIGITS {
                    self.message = Some("Too large".into());
                    self.toks = vec![Tok::Num(b)];
                    return;
                } else if digits > long::POW_SYNC_MAX_DIGITS {
                    self.pending_job =
                        Some(format!("pow:{}:{}", b.abs() as u64, e as u64));
                    self.toks = vec![Tok::Num(b)];
                    self.entry.clear();
                    return;
                } else if digits > 280.0 {
                    // beyond f64 range but small enough to compute inline
                    let nat = long::pow_big(b.abs() as u64, e as u64);
                    let neg = b < 0.0 && (e as u64) % 2 == 1;
                    let mut dec = nat.to_string();
                    if neg {
                        dec.insert(0, '-');
                    }
                    self.toks.clear();
                    let label = format!("{}^{}", trim_num(b), trim_num(e));
                    self.big_result = Some((long::approx_of(&dec), label));
                    self.full_result = Some(dec);
                    return;
                }
            }
        }
        let mut rep: Vec<Tok> = Vec::new();
        for t in self.toks.iter().rev() {
            match t {
                Tok::Op(c @ ('+' | '-' | '*' | '/' | '^' | 'r' | 'E')) => {
                    rep.insert(0, Tok::Op(*c));
                    break;
                }
                Tok::Num(_) | Tok::Fact | Tok::RParen => rep.insert(0, t.clone()),
                _ => break,
            }
        }
        self.last_repeat = if rep.len() >= 2 { Some(rep) } else { None };

        match self.eval_current() {
            Ok(v) if v.is_finite() => {
                self.toks = vec![Tok::Num(v)];
                self.entry.clear();
                self.just_eval = true;
            }
            _ => {
                self.error = true;
                self.toks.clear();
                self.entry.clear();
            }
        }
    }

    fn all_clear(&mut self) {
        self.toks.clear();
        self.entry.clear();
        self.just_eval = false;
        self.error = false;
        self.last_repeat = None;
    }

    fn clear_entry(&mut self) {
        self.entry.clear();
        self.error = false;
        self.just_eval = false;
    }

    fn backspace(&mut self) {
        if self.just_eval || self.error {
            return;
        }
        self.entry.pop();
    }

    fn negate(&mut self) {
        if self.error {
            return;
        }
        if self.entry.is_empty() {
            // negate whole evaluated expression → wrap -( expr )
            let v = self.current_value();
            self.toks.clear();
            self.toks.push(Tok::Num(-v));
        } else if let Some(stripped) = self.entry.strip_prefix('-') {
            self.entry = stripped.to_string();
        } else {
            self.entry = format!("-{}", self.entry);
        }
    }

    fn percent(&mut self) {
        if self.error {
            return;
        }
        if self.entry.is_empty() {
            return;
        }
        // Apple context-aware percent: A + B% → A + A*B/100
        let ctx = self.percent_context();
        if let Ok(mut n) = self.entry.parse::<f64>() {
            n = match ctx {
                Some(base) => base * n / 100.0,
                None => n / 100.0,
            };
            self.entry = trim_num(n);
        }
    }

    /// if expression ends with A + or A - , return A
    fn percent_context(&self) -> Option<f64> {
        match self.toks.last() {
            Some(Tok::Op('+')) | Some(Tok::Op('-')) => {
                let t = &self.toks[..self.toks.len() - 1];
                evaluate(t, self.deg).ok()
            }
            _ => None,
        }
    }

    fn lparen(&mut self) {
        if self.error || self.just_eval {
            self.toks.clear();
            self.just_eval = false;
            self.error = false;
        }
        if !self.entry.is_empty() {
            // implicit multiplication: 2( → 2*(
            self.commit_entry();
            self.toks.push(Tok::Op('*'));
        }
        self.toks.push(Tok::LParen);
    }

    fn rparen(&mut self) {
        if self.error {
            return;
        }
        self.commit_entry();
        // only close if a paren is open since last op boundary
        let mut depth = 0i32;
        for t in &self.toks {
            match t {
                Tok::LParen => depth += 1,
                Tok::RParen => depth -= 1,
                _ => {}
            }
        }
        if depth > 0
            && !matches!(
                self.toks.last(),
                Some(Tok::Op(_)) | Some(Tok::LParen) | Some(Tok::Func(_))
            )
        {
            self.toks.push(Tok::RParen);
        }
    }

    fn fact(&mut self) {
        if self.error {
            return;
        }
        if !self.entry.is_empty() {
            if let Ok(n) = self.entry.parse::<f64>() {
                if n >= 0.0 && n.fract() == 0.0 && n <= u64::MAX as f64 {
                    let n = n as u64;
                    if n <= long::BIGINT_FACT_MAX {
                        // fits f64 domain: instant
                        if let Ok(v) = factorial(n as f64) {
                            self.entry = trim_num(v);
                            return;
                        }
                    } else if n <= 1_000 {
                        // bigint, fast enough to run inline
                        let dec = long::factorial_big(n).to_string();
                        self.entry.clear();
                        self.toks.clear();
                        let label = format!("{n}!");
                        self.big_result = Some((long::approx_of(&dec), label));
                        self.full_result = Some(dec);
                        return;
                    } else if n <= long::LONG_FACT_MAX {
                        // long computation: handed to the worker
                        self.pending_job = Some(format!("fact:{n}"));
                        return;
                    } else {
                        self.entry.clear();
                        self.message = Some("Too large".into());
                        return;
                    }
                }
            }
            self.error = true;
            return;
        }
        // postfix only valid after a number or a closed paren
        if matches!(self.toks.last(), Some(Tok::Num(_)) | Some(Tok::RParen)) {
            self.toks.push(Tok::Fact);
        }
    }

    fn func(&mut self, name: &'static str) {
        if self.error {
            return;
        }
        if self.just_eval {
            // apply to result: wrap existing tokens
            self.just_eval = false;
        }
        if !self.entry.is_empty() {
            // apply to typed number: f(entry)
            if let Ok(n) = self.entry.parse::<f64>() {
                self.entry.clear();
                self.toks.push(Tok::Func(name));
                self.toks.push(Tok::LParen);
                self.toks.push(Tok::Num(n));
                self.toks.push(Tok::RParen);
            }
        } else {
            // apply to next operand: f( …user types… )
            // if last is RParen/Num → implicit multiply? Apple: wraps previous
            match self.toks.last() {
                Some(Tok::RParen) => {
                    // wrap entire last parenthesised group: find its '('
                    let mut depth = 0i32;
                    let mut idx = self.toks.len();
                    for (i, t) in self.toks.iter().enumerate().rev() {
                        match t {
                            Tok::RParen => depth += 1,
                            Tok::LParen => {
                                depth -= 1;
                                if depth == 0 {
                                    idx = i;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    self.toks.insert(idx, Tok::Func(name));
                }
                Some(Tok::Num(_)) => {
                    let n = self.toks.pop();
                    self.toks.push(Tok::Func(name));
                    self.toks.push(Tok::LParen);
                    if let Some(t) = n {
                        self.toks.push(t);
                    }
                    self.toks.push(Tok::RParen);
                }
                _ => {
                    self.toks.push(Tok::Func(name));
                    self.toks.push(Tok::LParen);
                }
            }
        }
    }

    fn constant(&mut self, v: f64) {
        if self.error || self.just_eval {
            self.toks.clear();
            self.just_eval = false;
            self.error = false;
        }
        if !self.entry.is_empty() {
            // number then constant → implicit multiply (Apple-style 2π)
            self.commit_entry();
            self.toks.push(Tok::Op('*'));
        }
        self.toks.push(Tok::Num(v));
    }

    fn rand(&mut self) {
        self.constant(random_f64());
    }

    fn mem_add(&mut self, sign: f64) {
        let v = if !self.entry.is_empty() {
            self.entry.parse().unwrap_or(0.0)
        } else {
            self.current_value()
        };
        self.memory += sign * v;
        self.has_memory = true;
    }
}

/// live formatting while typing: group integer digits, keep trailing '.'/'e'
fn format_entry(s: &str) -> String {
    let (mantissa, exp) = match s.split_once('e') {
        Some((m, e)) => (m, Some(e)),
        None => (s, None),
    };
    let neg = mantissa.starts_with('-');
    let body = if neg { &mantissa[1..] } else { mantissa };
    let (int_part, frac) = match body.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (body, None),
    };
    let mut out = String::new();
    if neg {
        out.push('-');
    }
    if int_part.is_empty() {
        out.push('0');
    } else {
        for (i, c) in int_part.chars().enumerate() {
            if i > 0 && (int_part.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
    }
    if let Some(f) = frac {
        out.push('.');
        out.push_str(f);
    }
    if let Some(e) = exp {
        out.push('e');
        out.push_str(e);
    }
    out
}

fn trim_num(v: f64) -> String {
    let s = format!("{}", v);
    if s.len() > 16 {
        format!("{:.10e}", v)
    } else {
        s
    }
}

fn tok_label(t: &Tok) -> String {
    match t {
        Tok::Num(n) => format_value(*n),
        Tok::Op('+') => " + ".into(),
        Tok::Op('-') => " − ".into(),
        Tok::Op('*') => " × ".into(),
        Tok::Op('/') => " ÷ ".into(),
        Tok::Op('^') => "^".into(),
        Tok::Op('r') => " ʸ√ ".into(),
        Tok::Op('E') => "ᴇ".into(),
        Tok::LParen => "(".into(),
        Tok::RParen => ")".into(),
        Tok::Func(f) => func_label(f).into(),
        Tok::Fact => "!".into(),
        _ => String::new(),
    }
}

fn func_label(f: &str) -> &'static str {
    match f {
        "asin" => "sin⁻¹",
        "acos" => "cos⁻¹",
        "atan" => "tan⁻¹",
        "asinh" => "sinh⁻¹",
        "acosh" => "cosh⁻¹",
        "atanh" => "tanh⁻¹",
        "exp" => "eˣ",
        "exp10" => "10ˣ",
        "exp2" => "2ˣ",
        "inv" => "1/x",
        "sq" => "x²",
        "cube" => "x³",
        "sqrt" => "√",
        "cbrt" => "∛",
        "log2" => "log₂",
        "log10" => "log",
        "abs" => "abs",
        "floor" => "floor",
        "ceil" => "ceil",
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "sinh" => "sinh",
        "cosh" => "cosh",
        "tanh" => "tanh",
        "ln" => "ln",
        _ => "fn",
    }
}

const FUNC_NAMES: &[&str] = &[
    "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "asinh", "acosh",
    "atanh", "ln", "log2", "log10", "sqrt", "cbrt", "exp", "exp10", "exp2", "inv", "sq",
    "cube", "abs", "floor", "ceil",
];

/// cryptographically-seeded uniform f64 in [0,1) via getrandom
fn random_f64() -> f64 {
    let mut b = [0u8; 8];
    getrandom::getrandom(&mut b).expect("rng unavailable");
    let n = u64::from_ne_bytes(b) >> 11; // 53 bits
    n as f64 / 9007199254740992.0 // 2^53
}
