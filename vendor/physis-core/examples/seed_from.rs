use rand::rngs::StdRng;
use rand::RngCore;
use rand::SeedableRng;
fn main() {
    // seed bytes from argv as hex
    let hex = std::env::args().nth(1).unwrap();
    let mut seed = [0u8; 32];
    for i in 0..32 {
        seed[i] = u8::from_str_radix(&hex[2*i..2*i+2], 16).unwrap();
    }
    let mut rng = StdRng::from_seed(seed);
    for _ in 0..4 { print!("{} ", rng.next_u32()); }
    println!();
}
