use cache_out_lib::clean::{clean_profile, CleanRequest, Key};
use cache_out_lib::model::Family;
use std::path::{Path, PathBuf};

fn show(label: &str, p: &Path, domain: &str) {
    let s = if p.join("places.sqlite").is_file() { cache_out_lib::firefox::scan(p) } else { cache_out_lib::chromium::scan(p) }.expect("scan");
    match s.sites.iter().find(|x| x.domain == domain) {
        Some(x) => println!("{label}: куки {} · адресов {} · визитов {} · загрузок {} · хранилище {} Б · кеш сайта {} Б", x.cookies, x.history_urls, x.visits, x.downloads, x.storage_bytes, x.site_cache_bytes),
        None => println!("{label}: {domain} нет в скане"),
    }
    println!("{label}: сайтов {}, кеш профиля {} Б, полей форм {}, адресов {}, занято {:?}", s.sites.len(), s.cache_bytes, s.forms.len(), s.addresses, s.locked);
}

fn main() {
    let mut args = std::env::args().skip(1);
    let profile = PathBuf::from(args.next().expect("путь к копии профиля"));
    let domain = args.next().expect("домен");
    let whole = args.next().is_some_and(|a| a == "--profile");
    show("до", &profile, &domain);
    let request = CleanRequest {
        sites: [(domain.clone(), vec![Key::Cookies, Key::History, Key::Downloads, Key::Storage, Key::SiteCache])].into(),
        profile: if whole { vec![Key::BrowserCache, Key::Forms, Key::Addresses] } else { vec![] },
    };
    let backup = profile.parent().unwrap().join("backup");
    let family = if profile.join("places.sqlite").is_file() { Family::Firefox } else { Family::Chromium };
    let report = clean_profile(&profile, family, &request, backup).expect("clean");
    println!("освобождено {} Б, копия {:?}", report.freed_bytes, report.backup);
    show("после", &profile, &domain);
}
