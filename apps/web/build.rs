//! Keep the shipped preview font in step with the bilingual story strings.
use std::{collections::BTreeSet, fs, path::Path};

fn required_ideographs(path: &Path, chars: &mut BTreeSet<char>) {
    println!("cargo:rerun-if-changed={}", path.display());
    if path.is_dir() {
        for entry in fs::read_dir(path).expect("read preview sources") {
            required_ideographs(&entry.expect("source entry").path(), chars);
        }
    } else if path.extension().is_some_and(|ext| ext == "rs") {
        chars.extend(fs::read_to_string(path).expect("UTF-8 Rust source").chars()
            .filter(|c| matches!(u32::from(*c), 0x3400..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x323af)));
    }
}
fn main() {
    let path = "www/fonts/KUIStorySans.ttf";
    println!("cargo:rerun-if-changed={path}");
    let bytes = fs::read(path).expect("bundled Chinese preview font");
    let face = ttf_parser::Face::parse(&bytes, 0).expect("valid preview font");
    let mut chars = BTreeSet::new();
    for dir in ["../story/src", "../../registry/src"] {
        required_ideographs(Path::new(dir), &mut chars);
    }
    let missing: String = chars
        .into_iter()
        .filter(|&c| face.glyph_index(c).is_none())
        .collect();
    assert!(
        missing.is_empty(),
        "Preview font missing {missing}. Regenerate it with scripts/subset-story-font.py and the OFL Noto Sans SC source."
    );
}
