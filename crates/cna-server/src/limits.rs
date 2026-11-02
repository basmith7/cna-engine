//! Open sign-up's abuse limits: per-IP sign-in starts, per-account
//! orders and game creations (each a keyed `governor` rate limiter), the
//! open-games cap, and the CSRF check on state-changing requests.

use crate::App;
use crate::games::AccountId;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;

/// The limits, the spec's numbers by default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub sign_ins_per_minute: u32,
    pub orders_per_minute: u32,
    pub games_per_hour: u32,
    /// Open games (waiting for a player) one account may have made.
    pub open_games: i64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            sign_ins_per_minute: 10,
            orders_per_minute: 60,
            games_per_hour: 10,
            open_games: 20,
        }
    }
}

/// The rate limiters, one bucket per key.
pub struct Limiters {
    pub limits: Limits,
    sign_ins: DefaultKeyedRateLimiter<IpAddr>,
    orders: DefaultKeyedRateLimiter<AccountId>,
    games: DefaultKeyedRateLimiter<AccountId>,
}

fn nz(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).unwrap_or(NonZeroU32::MIN)
}

impl Limiters {
    pub fn new(limits: Limits) -> Limiters {
        Limiters {
            sign_ins: RateLimiter::keyed(Quota::per_minute(nz(limits.sign_ins_per_minute))),
            orders: RateLimiter::keyed(Quota::per_minute(nz(limits.orders_per_minute))),
            games: RateLimiter::keyed(Quota::per_hour(nz(limits.games_per_hour))),
            limits,
        }
    }

    pub fn sign_in(&self, ip: IpAddr) -> bool {
        self.sign_ins.check_key(&ip).is_ok()
    }

    pub fn order(&self, account: AccountId) -> bool {
        self.orders.check_key(&account).is_ok()
    }

    pub fn create_game(&self, account: AccountId) -> bool {
        self.games.check_key(&account).is_ok()
    }

    /// Forgets the keys whose buckets are full again (run now and then,
    /// so the maps do not grow with every address ever seen).
    pub fn prune(&self) {
        self.sign_ins.retain_recent();
        self.orders.retain_recent();
        self.games.retain_recent();
    }
}

/// The client's address: the peer's, or with `trust_proxy` the last one
/// in `X-Forwarded-For` (the address the reverse proxy saw; earlier
/// entries are the client's own claims).
pub fn client_ip(headers: &header::HeaderMap, peer: SocketAddr, trust_proxy: bool) -> IpAddr {
    if trust_proxy
        && let Some(ip) = headers
            .get_all("x-forwarded-for")
            .iter()
            .next_back()
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.rsplit(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
    {
        return ip;
    }
    peer.ip()
}

/// `scheme://host[:port]` of a URL: what a browser sends as `Origin`.
fn origin_of(url: &str) -> &str {
    match url.find("://") {
        Some(i) => {
            let rest = &url[i + 3..];
            &url[..i + 3 + rest.find('/').unwrap_or(rest.len())]
        }
        None => url,
    }
}

/// CSRF: a request that may change state (anything but GET, HEAD and
/// OPTIONS) needs `Content-Type: application/json` and an `Origin`
/// equal to the public URL's; 403 otherwise. A cross-site form can do
/// neither.
pub async fn csrf(State(app): State<App>, req: Request, next: Next) -> Response {
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return next.run(req).await;
    }
    let h = req.headers();
    let json = h
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        .is_some_and(|v| v.trim().eq_ignore_ascii_case("application/json"));
    let ours = h
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|o| o == origin_of(&app.auth.public_url));
    if !(json && ours) {
        return crate::api::ApiError::new(
            StatusCode::FORBIDDEN,
            "a state change needs Content-Type: application/json and this site's Origin",
        )
        .into_response();
    }
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_of_a_url_drops_its_path() {
        assert_eq!(origin_of("https://cna.example/app/"), "https://cna.example");
        assert_eq!(origin_of("http://localhost:3240"), "http://localhost:3240");
    }

    #[test]
    fn a_forwarded_address_counts_only_behind_a_trusted_proxy() {
        let mut h = header::HeaderMap::new();
        h.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        let peer: SocketAddr = "9.9.9.9:1".parse().unwrap();
        assert_eq!(client_ip(&h, peer, false), peer.ip());
        assert_eq!(
            client_ip(&h, peer, true),
            "2.2.2.2".parse::<IpAddr>().unwrap()
        );
        assert_eq!(client_ip(&Default::default(), peer, true), peer.ip());
    }
}
