use taskboard_core::ActorKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub kind: ActorKind,
    pub label: String,
}
