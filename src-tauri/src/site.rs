use std::net::IpAddr;

pub fn site_of_host(host: &str) -> Option<String> {
    let host = host.trim().trim_start_matches('.').trim_end_matches('.').to_ascii_lowercase();
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if host.is_empty() {
        return None;
    }
    if host.parse::<IpAddr>().is_ok() || !host.contains('.') {
        return Some(host.to_string());
    }
    Some(icann_domain(host).or_else(|| psl::domain_str(host).map(String::from)).unwrap_or_else(|| host.to_string()))
}

fn icann_domain(host: &str) -> Option<String> {
    let labels: Vec<&str> = host.split('.').collect();
    (0..labels.len().saturating_sub(1)).find_map(|i| {
        let rest = labels[i + 1..].join(".");
        let suffix = psl::suffix(rest.as_bytes())?;
        (suffix.as_bytes() == rest.as_bytes() && suffix.typ() == Some(psl::Type::Icann)).then(|| labels[i..].join("."))
    })
}

pub fn site_of_url(raw: &str) -> Option<String> {
    let url = url::Url::parse(raw).ok()?;
    match url.scheme() {
        "http" | "https" | "ws" | "wss" => site_of_host(url.host_str()?),
        _ => None,
    }
}

pub fn site_of_origin(origin: &str) -> Option<String> {
    site_of_url(origin.split('^').next()?)
}

pub fn chrome_time_ms(t: i64) -> Option<i64> {
    (t > 0).then(|| t / 1000 - 11_644_473_600_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_by_registrable_domain() {
        assert_eq!(site_of_host(".accounts.google.com").as_deref(), Some("google.com"));
        assert_eq!(site_of_host("www.bbc.co.uk").as_deref(), Some("bbc.co.uk"));
        assert_eq!(site_of_host("someone.blogspot.com").as_deref(), Some("blogspot.com"));
        assert_eq!(site_of_host("stillmvd.github.io").as_deref(), Some("github.io"));
        assert_eq!(site_of_host("router.lan").as_deref(), Some("router.lan"));
        assert_eq!(site_of_host("localhost").as_deref(), Some("localhost"));
        assert_eq!(site_of_host("192.168.3.1").as_deref(), Some("192.168.3.1"));
        assert_eq!(site_of_host(""), None);
    }

    #[test]
    fn parses_urls_and_origins() {
        assert_eq!(site_of_url("https://m.vk.com/feed?x=1").as_deref(), Some("vk.com"));
        assert_eq!(site_of_url("chrome://settings"), None);
        assert_eq!(site_of_url("file:///C:/a.html"), None);
        assert_eq!(site_of_origin("https://embed.example.org/^0https://top.com").as_deref(), Some("example.org"));
    }

    #[test]
    fn converts_chrome_time() {
        assert_eq!(chrome_time_ms(13_000_000_000_000_000), Some(1_355_526_400_000));
        assert_eq!(chrome_time_ms(0), None);
    }
}
