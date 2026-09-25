#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbedChoice {
    CopyDist,
    HtmlShell,
}

pub fn decide(profile: &str, index_exists: bool) -> Result<EmbedChoice, &'static str> {
    if index_exists {
        Ok(EmbedChoice::CopyDist)
    } else if profile == "release" {
        Err("release build requires apps/web/dist/index.html; run pnpm --filter web build first")
    } else {
        Ok(EmbedChoice::HtmlShell)
    }
}
