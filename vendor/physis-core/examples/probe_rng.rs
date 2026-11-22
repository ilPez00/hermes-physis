use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
fn main() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut buf = [0u8; 64];
    rng.fill(&mut buf);
    print!("block0 bytes:");
    for b in &buf { print!(" {:02x}", b); }
    println!();
}
