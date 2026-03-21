//! Validate command — run post-transformation validation checks.

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::Args;
use tracing::info;

/// Run post-transformation validation checks.
#[derive(Args, Debug)]
#[command(about = "Run post-transformation validation checks on transformed code")]
pub struct ValidateArgs {
    /// Path to the file or directory to validate.
    #[arg(default_value = ".")]
    pub path: String,
}

/// Code file extensions to check during validation.
const CODE_EXTENSIONS: &[&str] = &[
    "py", "pyi", "ts", "tsx", "js", "jsx", "mjs", "java", "go", "tf", "hcl",
];

/// AWS import/SDK patterns to detect.
const AWS_PATTERNS: &[&str] = &[
    "import boto3",
    "from boto3",
    "import botocore",
    "from botocore",
    "require('aws-sdk')",
    "require(\"aws-sdk\")",
    "from '@aws-sdk/",
    "from \"@aws-sdk/",
    "@aws-sdk/",
    "import software.amazon.awssdk",
    "com.amazonaws",
    "aws_sdk_",
    "github.com/aws/aws-sdk-go",
];

/// Azure import/SDK patterns to detect.
const AZURE_PATTERNS: &[&str] = &[
    "from azure.",
    "import azure.",
    "require('@azure/",
    "require(\"@azure/",
    "from '@azure/",
    "from \"@azure/",
    "com.azure.",
    "import com.microsoft.azure",
    "azure-sdk",
    "github.com/Azure/azure-sdk-for-go",
];

/// Collect code files from a path (file or directory).
fn collect_code_files(path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if is_code_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(files);
    }

    if path.is_dir() {
        walk_dir(path, &mut files)?;
    }

    files.sort();
    Ok(files)
}

/// Recursively walk a directory and collect code files.
fn walk_dir(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden directories and common non-source dirs
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                || name == "node_modules"
                || name == "target"
                || name == "__pycache__"
                || name == "venv"
                || name == ".venv"
            {
                continue;
            }
        }

        if path.is_dir() {
            walk_dir(&path, files)?;
        } else if is_code_file(&path) {
            files.push(path);
        }
    }
    Ok(())
}

/// Check if a file is a code file based on its extension.
fn is_code_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| CODE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

/// Check for remaining cloud import patterns in file content.
/// Returns a list of matched import lines.
fn check_remaining_cloud_imports(content: &str, cloud: &str) -> Vec<String> {
    let patterns: &[&str] = match cloud {
        "aws" => AWS_PATTERNS,
        "azure" => AZURE_PATTERNS,
        _ => return Vec::new(),
    };

    let mut found = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        for pattern in patterns {
            if trimmed.contains(pattern) {
                found.push(trimmed.to_string());
                break; // Only report each line once
            }
        }
    }
    found
}

pub fn run(args: ValidateArgs) -> Result<()> {
    info!(path = %args.path, "Running validation");

    let path = Path::new(&args.path);
    if !path.exists() {
        bail!("Path not found: {}", args.path);
    }

    let files = collect_code_files(path)?;
    if files.is_empty() {
        println!("No code files found in {}", args.path);
        return Ok(());
    }

    println!("Validating {} code file(s) in {}\n", files.len(), args.path);

    let mut warnings = 0;
    let mut clean = 0;
    for file in &files {
        let content = std::fs::read_to_string(file)?;
        let remaining_aws = check_remaining_cloud_imports(&content, "aws");
        let remaining_azure = check_remaining_cloud_imports(&content, "azure");
        if !remaining_aws.is_empty() || !remaining_azure.is_empty() {
            println!("  WARN  {}", file.display());
            for imp in &remaining_aws {
                println!("        AWS import remains: {}", imp);
            }
            for imp in &remaining_azure {
                println!("        Azure import remains: {}", imp);
            }
            warnings += 1;
        } else {
            clean += 1;
        }
    }

    println!(
        "\nValidation: {} file(s) clean, {} with remaining cloud imports",
        clean, warnings
    );
    Ok(())
}
