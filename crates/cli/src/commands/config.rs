//! Config commands — read/write TOML configuration

use std::path::{Path, PathBuf};

/// Find the forge.toml file starting from current directory
pub fn find_config() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        let candidate = current.join("forge.toml");
        if candidate.exists() {
            return Some(candidate);
        }
        if !current.pop() {
            break;
        }
    }
    None
}

/// Show current configuration
pub fn show_config() -> anyhow::Result<()> {
    match find_config() {
        Some(path) => {
            let content = std::fs::read_to_string(&path)?;
            println!("  Config: {}", path.display());
            println!();
            print!("{}", content);
        }
        None => {
            println!("  No forge.toml found in current directory or parents.");
            println!("  Run 'project-x init <name>' to create a project.");
        }
    }
    Ok(())
}

/// Get a config value by key (dot notation: "project.name")
pub fn get_config(key: &str) -> anyhow::Result<()> {
    match find_config() {
        Some(path) => {
            let content = std::fs::read_to_string(&path)?;
            let value: toml::Value = toml::from_str(&content)?;

            // Navigate dot-separated keys
            let parts: Vec<&str> = key.split('.').collect();
            let mut current = &value;
            for part in &parts {
                match current.get(*part) {
                    Some(v) => current = v,
                    None => {
                        println!("Key '{}' not found", key);
                        return Ok(());
                    }
                }
            }

            match current {
                toml::Value::String(s) => println!("{}", s),
                toml::Value::Integer(i) => println!("{}", i),
                toml::Value::Float(f) => println!("{}", f),
                toml::Value::Boolean(b) => println!("{}", b),
                toml::Value::Array(a) => println!("{}", serde_json::to_string_pretty(a)?),
                toml::Value::Table(t) => println!("{}", serde_json::to_string_pretty(t)?),
                toml::Value::Datetime(d) => println!("{}", d),
            }
        }
        None => {
            println!("No forge.toml found");
        }
    }
    Ok(())
}

/// Set a config value by key
pub fn set_config(key: &str, value: &str) -> anyhow::Result<()> {
    let path = find_config().ok_or_else(|| anyhow::anyhow!("No forge.toml found"))?;
    let content = std::fs::read_to_string(&path)?;

    // Parse as TOML value
    let mut doc: toml::Value = toml::from_str(&content)?;

    // Navigate and set
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() == 1 {
        // Simple key at root level
        // Try to parse as appropriate type
        if let Ok(b) = value.parse::<bool>() {
            doc.as_table_mut().unwrap().insert(key.to_string(), toml::Value::Boolean(b));
        } else if let Ok(i) = value.parse::<i64>() {
            doc.as_table_mut().unwrap().insert(key.to_string(), toml::Value::Integer(i));
        } else if let Ok(f) = value.parse::<f64>() {
            doc.as_table_mut().unwrap().insert(key.to_string(), toml::Value::Float(f));
        } else {
            doc.as_table_mut().unwrap().insert(key.to_string(), toml::Value::String(value.to_string()));
        }
    } else {
        // Nested key
        let mut current = doc.as_table_mut().unwrap();
        for part in &parts[..parts.len()-1] {
            current = current.entry(*part)
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
                .as_table_mut()
                .unwrap();
        }
        let last = parts.last().unwrap();
        if let Ok(b) = value.parse::<bool>() {
            current.insert(last.to_string(), toml::Value::Boolean(b));
        } else if let Ok(i) = value.parse::<i64>() {
            current.insert(last.to_string(), toml::Value::Integer(i));
        } else if let Ok(f) = value.parse::<f64>() {
            current.insert(last.to_string(), toml::Value::Float(f));
        } else {
            current.insert(last.to_string(), toml::Value::String(value.to_string()));
        }
    }

    // Write back
    let new_content = toml::to_string_pretty(&doc)?;
    std::fs::write(&path, new_content)?;

    println!("  Set {} = {}", key, value);
    Ok(())
}

/// Remove a config key
pub fn unset_config(key: &str) -> anyhow::Result<()> {
    let path = find_config().ok_or_else(|| anyhow::anyhow!("No forge.toml found"))?;
    let content = std::fs::read_to_string(&path)?;
    let mut doc: toml::Value = toml::from_str(&content)?;

    let parts: Vec<&str> = key.split('.').collect();
    let mut current = doc.as_table_mut().unwrap();
    for part in &parts[..parts.len()-1] {
        current = current.entry(*part)
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .unwrap();
    }
    let last = parts.last().unwrap();
    current.remove(*last);

    let new_content = toml::to_string_pretty(&doc)?;
    std::fs::write(&path, new_content)?;

    println!("  Removed {}", key);
    Ok(())
}

/// Export config to file
pub fn export_config(output: &Path) -> anyhow::Result<()> {
    let path = find_config().ok_or_else(|| anyhow::anyhow!("No forge.toml found"))?;
    std::fs::copy(&path, output)?;
    println!("  Exported to {}", output.display());
    Ok(())
}

/// Import config from file
pub fn import_config(input: &Path) -> anyhow::Result<()> {
    // Validate the input file is valid TOML
    let content = std::fs::read_to_string(input)?;
    let _: toml::Value = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;

    let output = find_config().ok_or_else(|| anyhow::anyhow!("No forge.toml found"))?;
    std::fs::copy(input, &output)?;
    println!("  Imported from {}", input.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_config() {
        // This test only works if run from a directory with forge.toml
        // or if we create a temp directory
        let dir = std::env::temp_dir().join(format!("test-config-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("forge.toml"), "[project]\nname = \"test\"\n").unwrap();

        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let found = find_config();
        assert!(found.is_some());
        assert!(found.unwrap().ends_with("forge.toml"));

        std::env::set_current_dir(&original).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }
}