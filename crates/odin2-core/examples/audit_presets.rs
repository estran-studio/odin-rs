//! Audit récursif de vrais .odin: cargo run -p odin2-core --features std
//! --example audit_presets -- CHEMIN_USINE [--list-ignored]
//! Un champ n'est consommé que par une affectation typée du parseur.
use odin2_core::preset::{OdinPreset, ValueTree};
use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};

fn collect(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    if path.is_file() {
        if path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| extension.eq_ignore_ascii_case("odin")) {
            files.push(path.to_path_buf());
        }
    } else if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let kind = entry.file_type()?;
            // Ne suit pas les liens de dossiers (cycles possibles).
            if kind.is_dir() || kind.is_file() { collect(&entry.path(), files)?; }
        }
    } else {
        return Err(format!("Chemin introuvable: {}", path.display()).into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(arguments.next().ok_or("Usage: audit_presets CHEMIN_USINE [--list-ignored]")?);
    let mut list_ignored = false;
    for argument in arguments {
        if argument == "--list-ignored" { list_ignored = true; }
        else { return Err(format!("Argument inconnu: {}", argument.to_string_lossy()).into()); }
    }
    let mut files = Vec::new();
    collect(&root, &mut files)?;
    files.sort();
    if files.is_empty() { return Err("Aucun fichier .odin trouvé; audit refusé".into()); }
    let mut unread = 0usize;
    let mut errors = 0usize;
    let mut loaded = 0usize;
    let mut ignored = 0usize;
    let mut migrations = BTreeMap::<String, usize>::new();
    let mut ignored_reasons = BTreeMap::<String, usize>::new();
    for file in &files {
        let result = std::fs::read(file)
            .map_err(|error| error.to_string())
            .and_then(|bytes| ValueTree::from_bytes(&bytes).map_err(|error| error.to_string()))
            .and_then(|tree| OdinPreset::from_value_tree_audited(&tree).map_err(|error| error.to_string()));
        match result {
            Ok((_preset, audit)) => {
                loaded += audit.loaded_parameters;
                unread += audit.unread.len();
                ignored += audit.ignored_by_upstream.len();
                for name in audit.unread { println!("NON_LU\t{}\t{name}", file.display()); }
                for migration in audit.migrations { *migrations.entry(migration).or_default() += 1; }
                for reason in audit.ignored_by_upstream {
                    if list_ignored { println!("IGNORE_AMONT\t{}\t{reason}", file.display()); }
                    *ignored_reasons.entry(reason).or_default() += 1;
                }
            }
            Err(error) => { errors += 1; eprintln!("ERREUR\t{}\t{error}", file.display()); }
        }
    }
    for (migration, count) in migrations { println!("MIGRATION\t{count}\t{migration}"); }
    for (reason, count) in ignored_reasons { println!("IGNORE_AMONT_TOTAL\t{count}\t{reason}"); }
    println!("PRESETS={} PARAMETRES_CHARGES={loaded} NON_LUS={unread} IGNORES_AMONT={ignored} ERREURS={errors}", files.len());
    if unread != 0 || errors != 0 { return Err("Audit incomplet: voir les chemins ci-dessus".into()); }
    Ok(())
}
