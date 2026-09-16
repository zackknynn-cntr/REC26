use std::env;

pub const ROOT_DEFAULT: &str = "rec26.dedyn.io";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceArea {
    Namespace,
    Accounts,
    Api,
    Auth,
    Match,
    Rooms,
    Unknown,
}

impl ServiceArea {
    pub fn label(self) -> &'static str {
        match self {
            Self::Namespace => "NS",
            Self::Accounts => "ACCOUNTS",
            Self::Api => "API",
            Self::Auth => "AUTH",
            Self::Match => "MATCH",
            Self::Rooms => "ROOMS",
            Self::Unknown => "UNKNOWN",
        }
    }
}

pub fn root_domain() -> String {
    env::var("REC26_DOMAIN").unwrap_or_else(|_| ROOT_DEFAULT.to_owned())
}

pub fn url(subdomain: &str) -> String {
    format!("https://{}.{}", subdomain, root_domain())
}

pub fn area_for_host(host: &str) -> ServiceArea {
    let host = host.split(':').next().unwrap_or(host).to_ascii_lowercase();
    let root = root_domain();
    if host == format!("ns.{root}") { ServiceArea::Namespace }
    else if host == format!("accounts.{root}") { ServiceArea::Accounts }
    else if host == format!("api.{root}") { ServiceArea::Api }
    else if host == format!("auth.{root}") { ServiceArea::Auth }
    else if host == format!("match.{root}") { ServiceArea::Match }
    else if host == format!("rooms.{root}") { ServiceArea::Rooms }
    else { ServiceArea::Unknown }
}

// Current REC26 deployment has only these public hosts. Logical services that do
// not yet have their own hostname are deliberately aliased to an existing area.
pub fn service_url(service: &str) -> String {
    match service {
        "Accounts" => url("accounts"),
        "Auth" => url("auth"),
        "Matchmaking" => url("match"),
        "Rooms" => url("rooms"),
        // API currently owns the remaining HTTP compatibility endpoints.
        _ => url("api"),
    }
}
