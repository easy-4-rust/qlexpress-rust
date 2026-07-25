/// How to manage string interpolation, e.g. `"a ${t-c} b"`, mirroring Java
/// `InterpolationMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterpolationMode {
    /// Implement interpolation using a QLExpress script. (Java default.)
    #[default]
    Script,
    /// Implement interpolation using a variable name in the context.
    Variable,
    /// Disable interpolation; `${xxx}` is rendered verbatim.
    Disable,
}
