use macrokid::{make_enum, trace};
use custom_derive::{Display, DebugVerbose, Display2, FirstExposed, AssocImpl, DisplayDSL};
use custom_derive_support::AssocDemo;

// Function-like macro: generates an enum with Display + FromStr
make_enum!(Color: Red, Green, Blue);

// Derive macro: implement Display for this enum (prints variant names)
#[derive(Debug, Display, Clone, Copy)]
enum Mode {
    Fast,
    #[display("SLOW")]
    Slow,
}

#[derive(Display)]
#[display("Point2D")]
struct Point(i32, i32);

// Demonstrate the more advanced DebugVerbose derive
#[derive(DebugVerbose)]
#[debug_verbose("CustomConfig")]
struct Config {
    name: String,
    #[skip]
    secret_key: String,
    port: u16,
}

// Attribute macro: times the function and logs duration
#[trace]
fn work(mode: Mode) -> usize {
    match mode {
        Mode::Fast => 1 + 1,
        Mode::Slow => {
            // Simulate some extra work
            let mut acc = 0;
            for i in 0..1_000 {
                acc += i % 3;
            }
            acc
        }
    }
}

fn main() {
    // Function-like macro: Display on generated enum
    let c: Color = "Green".parse().expect("valid variant");
    println!("Color from str: {}", c);

    // Custom derive macro: Display on our hand-written enum and struct
    println!("Mode: {}", Mode::Fast);
    println!("Mode (custom): {}", Mode::Slow);
    println!("Struct display: {}", Point(1, 2));
    // Touch Point fields so they are not considered dead code in this demo
    let p = Point(3, 4);
    let Point(a, b) = p;
    let _sum = a + b;

    // Advanced custom derive: DebugVerbose 
    let config = Config {
        name: "MyApp".to_string(),
        secret_key: "super_secret".to_string(), // This will be skipped in debug
        port: 8080,
    };
    println!("Config (verbose debug): {:?}", config);
    // Ensure skipped field participates in runtime to avoid dead code
    let _sk_len = config.secret_key.len();

    // Attribute macro: trace function execution
    let n = work(Mode::Slow);
    println!("work returned {}", n);

    // Demonstrate Display2 using semantic helper
    #[derive(Display2)]
    enum State { One, Two }
    println!("State: {} / {}", State::One, State::Two);

    // Demonstrate DisplayDSL (pattern DSL) for enums
    #[derive(DisplayDSL)]
    enum Kind { A, B(i32), C { x: i32 } }
    println!("Kind: {} / {} / {}", Kind::A, Kind::B(1), Kind::C { x: 2 });
    // Touch Kind variant fields to avoid dead code
    if let Kind::B(v) = Kind::B(7) { let _ = v; }
    if let Kind::C { x } = (Kind::C { x: 9 }) { let _ = x; }

    // Demonstrate FirstExposed for named-field struct
    #[derive(FirstExposed, Debug)]
    struct User {
        id: u32,
        #[expose]
        name: &'static str,
    }

    let user = User { id: 1, name: "Alice" };
    let _ = user.id;
    if let Some((key, val)) = user.first_exposed() {
        println!("first_exposed: {} = {:?}", key, val);
    }

    // Demonstrate ImplBuilder associated items via AssocImpl derive
    #[derive(AssocImpl)]
    struct AssocHolder;
    println!("AssocDemo COUNT = {}", <AssocHolder as AssocDemo>::COUNT);
    let out: <AssocHolder as AssocDemo>::Output = AssocHolder.get();
    println!("AssocDemo get() -> {}", out);
}
