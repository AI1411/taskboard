#[path = "../src/embed_policy.rs"]
mod embed_policy;

use embed_policy::{decide, EmbedChoice};

#[test]
fn release_without_dist_is_an_error() {
    let err = decide("release", false).unwrap_err();
    assert!(err.contains("apps/web/dist/index.html"));
}

#[test]
fn debug_without_dist_uses_the_blank_shell() {
    assert_eq!(decide("debug", false).unwrap(), EmbedChoice::HtmlShell);
}

#[test]
fn any_profile_with_dist_copies_it() {
    assert_eq!(decide("release", true).unwrap(), EmbedChoice::CopyDist);
    assert_eq!(decide("debug", true).unwrap(), EmbedChoice::CopyDist);
}
