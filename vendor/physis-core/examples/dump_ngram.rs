//! Dumps embedder vectors for fixed inputs — the oracle for cphysis's
//! C port (byte-exact parity fixtures). Usage:
//!   dump_ngram <ngram|rp> <dim> <seed> <text>...
use physis_core::embed::VectorEmbed;
use physis_core::embed_ngram::SyntheticNGramEmbedder;
use physis_core::embed::RandomProjectionEmbedder;

fn main() {
    let kind = std::env::args().nth(1).unwrap();
    let dim: usize = std::env::args().nth(2).unwrap().parse().unwrap();
    let seed: u64 = std::env::args().nth(3).unwrap().parse().unwrap();
    for text in std::env::args().skip(4) {
        let v = match kind.as_str() {
            "ngram" => SyntheticNGramEmbedder::new(dim, seed).embed(&text),
            "rp" => RandomProjectionEmbedder::new(dim).embed(&text),
            _ => panic!("kind must be ngram|rp"),
        };
        // exact f32 bit patterns — decimal prints cannot round-trip f32
        let vals: Vec<String> = v
            .iter()
            .map(|x| format!("{:08x}", x.to_bits()))
            .collect();
        println!("{} {}", text, vals.join(" "));
    }
}
