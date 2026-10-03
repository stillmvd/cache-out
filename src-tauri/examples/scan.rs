fn mb(b: u64) -> String {
    format!("{:.1} МБ", b as f64 / 1_048_576.0)
}

fn main() {
    let id = std::env::args().nth(1).unwrap_or_else(|| "edge".into());
    for b in cache_out_lib::browsers::detect() {
        println!("{} [{}]: {}", b.name, b.id, b.profiles.iter().map(|p| format!("{} ({})", p.name, p.id)).collect::<Vec<_>>().join(", "));
    }
    let Some(b) = cache_out_lib::browsers::detect().into_iter().find(|b| b.id == id) else { return };
    let p = &b.profiles[0];
    let t = std::time::Instant::now();
    match cache_out_lib::scan_profile_blocking(&b.id, &p.id) {
        Ok(s) => {
            println!("\n{} / {} — {} сайтов, кеш {}, полей форм {}, адресов {}, занято: {:?}, теневая копия: {}, {:?}",
                b.name, p.name, s.sites.len(), mb(s.cache_bytes), s.forms.len(), s.addresses, s.locked, s.from_shadow, t.elapsed());
            let mut top = s.sites.clone();
            top.sort_by_key(|x| std::cmp::Reverse(x.storage_bytes + x.cookies as u64 * 1000 + x.visits as u64 * 1000));
            for x in top.iter().take(8) {
                println!("  {:<28} куки {:>4}  визитов {:>5}  загрузок {:>3}  хранилище {}", x.domain, x.cookies, x.visits, x.downloads, mb(x.storage_bytes));
            }
        }
        Err(e) => println!("ошибка: {e}"),
    }
}
