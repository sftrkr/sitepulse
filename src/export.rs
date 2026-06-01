use crate::models::UrlCheckResult;
use anyhow::{Context, Result};
use std::path::Path;

pub fn export_csv(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)
        .with_context(|| format!("failed to create CSV file: {}", path.display()))?;

    for result in results {
        writer.serialize(result)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn export_json(path: &Path, results: &[UrlCheckResult]) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("failed to create JSON file: {}", path.display()))?;
    serde_json::to_writer_pretty(file, results)
        .with_context(|| format!("failed to write JSON file: {}", path.display()))?;
    Ok(())
}
