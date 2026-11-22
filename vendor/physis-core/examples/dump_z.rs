use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
fn main() {
    let text = "relay calibration bench harness";
    let mut h = Sha256::new();
    h.update(text.as_bytes());
    let result = h.finalize();
    let key = u64::from_le_bytes(result[..8].try_into().unwrap());
    let seed = 7u64 ^ key;
    let mut rng = StdRng::seed_from_u64(seed);
    let mut z = Vec::new();
    for _ in 0..384 {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        let zv = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        z.push(zv as f32);
    }
    let norm: f32 = z.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-8);
    println!("key={} seed={}", key, seed);
    println!("norm={:08x} sum={:08x}", norm.to_bits(), z.iter().map(|x| x * x).sum::<f32>().to_bits());
    for i in [0usize, 7, 11, 29, 52] {
        println!("z[{}]={:08x} -> {:08x}", i, z[i].to_bits(), (z[i] / norm).to_bits());
    }
}
