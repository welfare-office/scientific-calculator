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
fn long_job_factorial_and_resume() {
    let mut c = Calculator::new();
    // 500! routes to the synchronous bigint path
    run(&mut c, &["5", "0", "0", "fact"]);
    let d = c.display();
    assert!(d.ends_with("e1134"), "display was {d}");
    assert!(c.has_big_result());
    assert_eq!(c.tape(), "500!");
    let full = c.full_result();
    assert_eq!(full.len(), 1135); // 500! has 1135 digits
    assert!(full.starts_with("12201368259911100687012387"));
}

#[test]
fn long_job_factorial_routes_async() {
    let mut c = Calculator::new();
    run(&mut c, &["5", "0", "0", "0", "0", "fact"]);
    assert_eq!(c.take_long_job(), "fact:50000");
    assert_eq!(c.take_long_job(), ""); // drained
}

#[test]
fn long_job_power_detection() {
    let mut c = Calculator::new();
    // 2^1000000 -> async job
    run(&mut c, &["2", "pow", "1", "0", "0", "0", "0", "0", "0", "eq"]);
    assert_eq!(c.take_long_job(), "pow:2:1000000");

    // 9^300 -> 287 digits: beyond f64 but computed synchronously
    let mut c = Calculator::new();
    run(&mut c, &["9", "pow", "3", "0", "0", "eq"]);
    assert!(c.has_big_result());
    let d = c.display();
    assert!(d.ends_with("e286"), "display was {d}");
    assert_eq!(c.full_result().len(), 287);

    // astronomically large -> refused
    let mut c = Calculator::new();
    run(&mut c, &["9", "pow", "9", "0", "0", "0", "0", "0", "0", "0", "eq"]);
    assert_eq!(c.display(), "Too large");
    assert_eq!(c.take_long_job(), "");
}

#[test]
fn long_job_checkpoint_roundtrip() {
    use calculator_engine::long::{factorial_big, LongJob};
    let mut j = LongJob::new_factorial(2_000);
    j.step();
    j.step();
    let bytes = j.checkpoint();
    let mut resumed = LongJob::restore(&bytes).unwrap();
    while !resumed.done() {
        resumed.step();
    }
    assert_eq!(resumed.result().to_string(), factorial_big(2_000).to_string());

    // pow job roundtrip
    let mut p = LongJob::new_pow(3, 5_000);
    for _ in 0..3 {
        p.step();
    }
    let bytes = p.checkpoint();
    let mut p2 = LongJob::restore(&bytes).unwrap();
    while !p2.done() {
        p2.step();
    }
    let mut expect = LongJob::new_pow(3, 5_000);
    while !expect.done() {
        expect.step();
    }
    assert_eq!(p2.result().to_string(), expect.result().to_string());
}

#[test]
fn big_result_consumed_by_next_input() {
    let mut c = Calculator::new();
    run(&mut c, &["5", "0", "0", "fact"]); // 500! big result
    // seed is ~inf -> binary op ignored, digit starts fresh
    run(&mut c, &["1", "add", "1", "eq"]);
    assert_eq!(c.display(), "2");
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
