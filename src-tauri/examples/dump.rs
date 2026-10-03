fn main() {
    let mut args = std::env::args().skip(1);
    let id = args.next().unwrap_or_else(|| "chrome".into());
    let out = args.next().unwrap_or_else(|| "scan.json".into());
    let b = cache_out_lib::browsers::detect().into_iter().find(|b| b.id == id).expect("browser");
    let p = &b.profiles[0];
    let mut s = cache_out_lib::scan_profile_blocking(&b.id, &p.id).expect("scan");
    s.forms.truncate(12);
    let json = serde_json::json!({ "browser": b.name, "profile": p.name, "scan": s });
    std::fs::write(out, serde_json::to_string_pretty(&json).unwrap()).unwrap();
}
