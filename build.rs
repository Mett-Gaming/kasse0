//! Bettet Waren und Figuren ein, setzt die Version (Git-Tag) und das Windows-Manifest.
//! Die Shader baut Mettgine selbst (mettgine/build.rs).

use std::{env, fs, path::PathBuf};

/// Erzeugt OUT_DIR/products_gen.rs: Katalog-JSON + alle Waren-Modelle eingebettet.
fn gen_products(out: &std::path::Path) {
    println!("cargo:rerun-if-changed=tools/catalog.json");
    println!("cargo:rerun-if-changed=assets/products");
    let root = env::current_dir().unwrap();
    let catalog_path = root.join("tools").join("catalog.json");
    let text = fs::read_to_string(&catalog_path).expect("tools/catalog.json fehlt");
    let json: serde_json::Value = serde_json::from_str(&text).expect("catalog.json ungueltig");
    let mut ids = String::new();
    let mut bytes = String::new();
    for m in json["meshes"].as_array().expect("meshes") {
        let id = m["id"].as_str().expect("mesh id");
        let file = root
            .join("assets")
            .join("products")
            .join(format!("{id}.k0m"));
        assert!(
            file.exists(),
            "Modell fehlt: {} (tools/prepare_products.py ausfuehren)",
            file.display()
        );
        ids.push_str(&format!("    {id:?},\n"));
        bytes.push_str(&format!(
            "    include_bytes!({:?}),\n",
            file.display().to_string()
        ));
    }
    let code = format!(
        "pub const CATALOG_JSON: &str = include_str!({:?});\n\
         pub const PRODUCT_MESH_IDS: &[&str] = &[\n{ids}];\n\
         pub static PRODUCT_MESH_BYTES: &[&[u8]] = &[\n{bytes}];\n",
        catalog_path.display().to_string()
    );
    fs::write(out.join("products_gen.rs"), code).unwrap();
}

/// Figuren aus tools/people.json (Skinning): je Variante eine .k0s, je Geschlecht eine .k0a.
fn gen_people(out: &std::path::Path) {
    println!("cargo:rerun-if-changed=tools/people.json");
    println!("cargo:rerun-if-changed=assets/people");
    let root = env::current_dir().unwrap();
    let dir = root.join("assets").join("people");
    let text = fs::read_to_string(root.join("tools").join("people.json"))
        .expect("tools/people.json fehlt");
    let json: serde_json::Value = serde_json::from_str(&text).expect("people.json ungueltig");
    let need = |name: String| {
        let file = dir.join(&name);
        assert!(
            file.exists(),
            "Figur fehlt: {} (tools/prepare_people.py ausfuehren)",
            file.display()
        );
        format!("include_bytes!({:?})", file.display().to_string())
    };
    let mut variants = String::new();
    let mut skins = String::new();
    for v in json["variants"].as_array().expect("variants") {
        let id = v["id"].as_str().expect("variant id");
        let female = v["g"].as_str() == Some("f");
        let staff = v["staff"].as_bool().unwrap_or(false);
        variants.push_str(&format!("    ({id:?}, {female}, {staff}),\n"));
        skins.push_str(&format!("    {},\n", need(format!("{id}.k0s"))));
    }
    let code = format!(
        "/// (Variante, weiblich, Mitarbeiter)\npub const PEOPLE: &[(&str, bool, bool)] = &[\n{variants}];\n\
         pub static PEOPLE_SKIN_BYTES: &[&[u8]] = &[\n{skins}];\n\
         pub static ANIM_M_BYTES: &[u8] = {};\n\
         pub static ANIM_F_BYTES: &[u8] = {};\n",
        need("anim_m.k0a".into()),
        need("anim_f.k0a".into())
    );
    fs::write(out.join("people_gen.rs"), code).unwrap();
}

/// Version fuer Absturzberichte: in der CI der Tag, sonst `git describe`, sonst Cargo.toml.
fn gen_version() {
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/tags");
    let version = env::var("GITHUB_REF_NAME")
        .ok()
        .filter(|r| r.starts_with('v'))
        .or_else(|| {
            std::process::Command::new("git")
                .args(["describe", "--tags", "--always"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=KASSE0_VERSION={version}");
    // Absturzfenster: moderne Windows-Bedienelemente (Common Controls 6)
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}

fn main() {
    gen_version();
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    gen_products(&out);
    gen_people(&out);
}
