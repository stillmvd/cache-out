use rusty_leveldb::{LdbIterator, Options, DB};
use std::collections::HashMap;

fn main() {
    let dir = std::env::args().nth(1).expect("путь к копии Local Storage\\leveldb");
    let mut db = DB::open(&dir, Options { create_if_missing: false, ..Options::default() }).expect("open");
    let mut it = db.new_iter().expect("iter");
    let mut kinds: HashMap<String, (u32, u64)> = HashMap::new();
    let mut origins: HashMap<String, u64> = HashMap::new();
    let mut samples = vec![];
    while let Some((k, v)) = it.next() {
        let kind = k.iter().position(|b| *b == b':' || *b == 0).map_or(k.len().min(12), |n| n.min(12));
        let e = kinds.entry(String::from_utf8_lossy(&k[..kind]).to_string()).or_default();
        e.0 += 1;
        e.1 += (k.len() + v.len()) as u64;
        if k.first() == Some(&b'_') {
            let end = k.iter().position(|b| *b == 0).unwrap_or(k.len());
            *origins.entry(String::from_utf8_lossy(&k[1..end]).to_string()).or_default() += (k.len() + v.len()) as u64;
        }
        if samples.len() < 12 && !k.starts_with(b"_") {
            samples.push(String::from_utf8_lossy(&k[..k.len().min(70)]).to_string());
        }
    }
    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|x| std::cmp::Reverse(x.1 .1));
    println!("виды ключей: {kinds:?}");
    println!("примеры: {samples:?}");
    let mut o: Vec<_> = origins.into_iter().collect();
    o.sort_by_key(|x| std::cmp::Reverse(x.1));
    println!("origin всего {}, сумма {} КБ", o.len(), o.iter().map(|x| x.1).sum::<u64>() / 1024);
    for (k, n) in o.iter().take(8) {
        println!("  {k} {} КБ", n / 1024);
    }
}
