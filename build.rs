//! Kompiliert alle `shaders/*.wgsl` mit naga zu SPIR-V (kein Vulkan SDK noetig).
//! Jede Datei braucht die Entry Points `vs_main` und `fs_main`; erzeugt werden
//! `<name>.vert.spv` und `<name>.frag.spv` in OUT_DIR.

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

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    gen_products(&out);
    gen_people(&out);
    println!("cargo:rerun-if-changed=shaders");

    for entry in fs::read_dir("shaders").expect("Ordner shaders/ fehlt") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("wgsl") {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path.file_stem().unwrap().to_str().unwrap().to_owned();
        if name.starts_with('_') {
            continue; // nur zum Einbinden gedacht
        }
        // Minimaler Praeprozessor: `#include "datei.wgsl"`
        let src: String = fs::read_to_string(&path)
            .unwrap()
            .lines()
            .map(|line| match line.trim().strip_prefix("#include ") {
                Some(inc) => fs::read_to_string(format!("shaders/{}", inc.trim_matches('"')))
                    .unwrap_or_else(|_| panic!("{name}: include {inc} fehlt")),
                None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");

        let module = naga::front::wgsl::parse_str(&src).unwrap_or_else(|e| {
            panic!(
                "{}",
                e.emit_to_string_with_path(&src, path.to_str().unwrap())
            )
        });
        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{name}: {}", e.emit_to_string(&src)));

        for (stage, entry_point, suffix) in [
            (naga::ShaderStage::Vertex, "vs_main", "vert"),
            (naga::ShaderStage::Fragment, "fs_main", "frag"),
        ] {
            let options = naga::back::spv::Options::default();
            let pipeline = naga::back::spv::PipelineOptions {
                shader_stage: stage,
                entry_point: entry_point.to_string(),
            };
            let words = naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline))
                .unwrap_or_else(|e| panic!("{name} {entry_point}: {e}"));
            let bytes: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
            fs::write(out.join(format!("{name}.{suffix}.spv")), bytes).unwrap();
        }
    }
}
