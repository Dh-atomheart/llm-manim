pub const MANIM_CE_TARGET_VERSION: &str = "0.20.1";
pub const API_MANIFEST_JSON: &str =
    include_str!("../../../references/manimce/0.20.1/api_manifest.json");
pub const DENYLIST_JSON: &str = include_str!("../../../references/manimce/0.20.1/denylist.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_and_denylist_are_available() {
        assert!(API_MANIFEST_JSON.contains("\"allowedNames\""));
        assert!(DENYLIST_JSON.contains("\"deniedNames\""));
        assert!(DENYLIST_JSON.contains("ParametricSurface"));
    }
}
