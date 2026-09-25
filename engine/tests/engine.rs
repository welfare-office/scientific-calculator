use calculator_engine::Calculator;

fn run(c: &mut Calculator, keys: &[&str]) -> String {
    for k in keys {
        c.press(k);
    }
    c.display()
}

#[test]
fn basic_arithmetic() {
    let mut c = Calculator::new();
    assert_eq!(run(&mut c, &["1", "2", "add", "3", "eq"]), "15");
    let mut c = Calculator::new();
    assert_eq!(run(&mut c, &["2", "add", "3", "mul", "4", "eq"]), "14");
    let mut c = Calculator::new();
    assert_eq!(run(&mut c, &["1", "0", "div", "4", "eq"]), "2.5");
}

#[test]
fn chaining_shows_partial() {
    let mut c = Calculator::new();
    c.press("1");
    c.press("0");
    c.press("add");
    c.press("5");
    assert_eq!(c.display(), "5");
    c.press("mul"); // evaluates 10+5 → shows 15
    assert_eq!(c.display(), "15");
}

#[test]
fn percent_context() {
    let mut c = Calculator::new();
    // 200 + 10% → 220 (Apple behavior)
    assert_eq!(
        run(&mut c, &["2", "0", "0", "add", "1", "0", "pct", "eq"]),
        "220"
    );
}

#[test]
fn scientific_funcs() {
    let mut c = Calculator::new();
    run(&mut c, &["9", "sqrt"]);
    assert_eq!(c.display(), "3");
    let mut c = Calculator::new();
    run(&mut c, &["2", "sq", "eq"]);
    assert_eq!(c.display(), "4");
    let mut c = Calculator::new();
    run(&mut c, &["2", "pow", "1", "0", "eq"]);
    assert_eq!(c.display(), "1,024");
    let mut c = Calculator::new();
    run(&mut c, &["5", "fact", "eq"]);
    assert_eq!(c.display(), "120");
}

#[test]
fn trig_deg_rad() {
    let mut c = Calculator::new();
    run(&mut c, &["9", "0", "sin", "eq"]);
    assert_eq!(c.display(), "1"); // sin(90°)
    let mut c = Calculator::new();
    c.press("rad");
    c.press("pi");
    c.press("sin");
    let v: f64 = c.display().replace(',', "").parse().unwrap_or(1.0);
    assert!(v.abs() < 1e-9);
}

#[test]
fn parens_and_errors() {
    let mut c = Calculator::new();
    run(&mut c, &["lparen", "2", "add", "3", "rparen", "mul", "4", "eq"]);
    assert_eq!(c.display(), "20");
    let mut c = Calculator::new();
    run(&mut c, &["1", "div", "0", "eq"]);
    assert_eq!(c.display(), "Error");
    let mut c = Calculator::new();
    run(&mut c, &["1", "div", "0", "eq", "ac", "7"]);
    assert_eq!(c.display(), "7");
}

#[test]
fn memory_and_repeat_eq() {
    let mut c = Calculator::new();
    run(&mut c, &["4", "2", "m+"]);
    assert!(c.has_memory());
    c.press("ac");
    c.press("mr");
    assert_eq!(c.display(), "42");
    // repeated '='
    let mut c = Calculator::new();
    run(&mut c, &["3", "add", "2", "eq"]);
    assert_eq!(c.display(), "5");
    c.press("eq");
    assert_eq!(c.display(), "7");
}
