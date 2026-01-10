//! File Manager Example
//!
//! Demonstrates file operations and validation with dynamic-cli.
//! Shows how to use path validation, file extension checking, and formatted output.
//!
//! # Usage
//!
//! ```bash
//! # REPL mode
//! cargo run --example file_manager
//!
//! # CLI mode
//! cargo run --example file_manager -- list /path/to/dir
//! cargo run --example file_manager -- info myfile.txt
//! cargo run --example file_manager -- search /path/to/dir --pattern "*.rs"
//! ```

use dynamic_cli::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ============================================================================
// EXECUTION CONTEXT
// ============================================================================

/// File manager context stores current directory and statistics
#[derive(Default)]
struct FileManagerContext {
    current_dir: Option<PathBuf>,
    files_processed: usize,
    last_operation: Option<String>,
}

impl ExecutionContext for FileManagerContext {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

/// Handler for list command
struct ListCommand;

impl CommandHandler for ListCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<FileManagerContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "FileManagerContext".to_string(),
                })
            })?;

        // Get directory path
        let dir = args.get("directory").map(|s| s.as_str()).unwrap_or(".");
        let path = Path::new(dir);

        // Validate directory exists
        if !path.exists() {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::FileNotFound {
                    path: path.to_path_buf(),
                    arg_name: "directory".to_string(),
                }
            ));
        }

        if !path.is_dir() {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "directory".to_string(),
                    reason: "Path must be a directory".to_string(),
                }
            ));
        }

        // Read directory
        let entries = fs::read_dir(path).map_err(DynamicCliError::from)?;
        
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut total_size = 0u64;

        for entry in entries {
            let entry = entry.map_err(DynamicCliError::from)?;
            let metadata = entry.metadata().map_err(DynamicCliError::from)?;
            let name = entry.file_name().to_string_lossy().to_string();

            if metadata.is_dir() {
                dirs.push(format!("{}/", name));
            } else {
                let size = metadata.len();
                total_size += size;
                files.push(format!("{} ({})", name, 
                    dynamic_cli::utils::format_bytes(size)));
            }
        }

        // Update context
        ctx.current_dir = Some(path.to_path_buf());
        ctx.files_processed += files.len() + dirs.len();
        ctx.last_operation = Some("list".to_string());

        // Display results
        println!("\nDirectory: {}", path.display());
        println!("─────────────────────────────────────");
        
        if !dirs.is_empty() {
            println!("\nDirectories:");
            println!("{}", dynamic_cli::utils::format_numbered_list(&dirs));
        }
        
        if !files.is_empty() {
            println!("\nFiles:");
            println!("{}", dynamic_cli::utils::format_numbered_list(&files));
        }
        
        println!("\n─────────────────────────────────────");
        println!("Total: {} directories, {} files ({})", 
            dirs.len(), files.len(), dynamic_cli::utils::format_bytes(total_size));

        Ok(())
    }
}

/// Handler for info command
struct InfoCommand;

impl CommandHandler for InfoCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<FileManagerContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "FileManagerContext".to_string(),
                })
            })?;

        let file = args.get("file").unwrap();
        let path = Path::new(file);

        // Validate file exists
        if !path.exists() {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::FileNotFound {
                    path: path.to_path_buf(),
                    arg_name: "file".to_string(),
                }
            ));
        }

        // Get metadata
        let metadata = fs::metadata(path).map_err(DynamicCliError::from)?;
        
        // Update context
        ctx.files_processed += 1;
        ctx.last_operation = Some("info".to_string());

        // Display info
        println!("\nFile Information");
        println!("═════════════════════════════════════");
        println!("Path:       {}", path.display());
        println!("Type:       {}", if metadata.is_dir() { "Directory" } else { "File" });
        println!("Size:       {}", dynamic_cli::utils::format_bytes(metadata.len()));
        
        if let Some(ext) = dynamic_cli::utils::get_extension(file) {
            println!("Extension:  .{}", ext);
        }
        
        if let Ok(modified) = metadata.modified() {
            println!("Modified:   {:?}", modified);
        }
        
        println!("Permissions: {:?}", metadata.permissions());
        println!("Read-only:  {}", metadata.permissions().readonly());

        Ok(())
    }
}

/// Handler for search command
struct SearchCommand;

impl CommandHandler for SearchCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_mut::<FileManagerContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "FileManagerContext".to_string(),
                })
            })?;

        let dir = args.get("directory").map(|s| s.as_str()).unwrap_or(".");
        let pattern = args.get("pattern").map(|s| s.as_str()).unwrap_or("*");
        
        let path = Path::new(dir);

        // Validate directory
        if !path.exists() || !path.is_dir() {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "directory".to_string(),
                    reason: "Must be an existing directory".to_string(),
                }
            ));
        }

        // Simple pattern matching (just extension for now)
        let extension = if pattern.starts_with("*.") {
            Some(&pattern[2..])
        } else {
            None
        };

        // Search files
        let mut matches = Vec::new();
        
        for entry in fs::read_dir(path).map_err(DynamicCliError::from)? {
            let entry = entry.map_err(DynamicCliError::from)?;
            let name = entry.file_name().to_string_lossy().to_string();
            
            let is_match = if let Some(ext) = extension {
                dynamic_cli::utils::has_extension(&name, &[ext])
            } else {
                name.contains(pattern)
            };
            
            if is_match {
                matches.push(name);
            }
        }

        // Update context
        ctx.files_processed += matches.len();
        ctx.last_operation = Some("search".to_string());

        // Display results
        println!("\nSearch Results in {}", path.display());
        println!("Pattern: {}", pattern);
        println!("─────────────────────────────────────");
        
        if matches.is_empty() {
            println!("No files found.");
        } else {
            println!("\nFound {} file(s):", matches.len());
            println!("{}", dynamic_cli::utils::format_numbered_list(&matches));
        }

        Ok(())
    }
}

/// Handler for stats command
struct StatsCommand;

impl CommandHandler for StatsCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = dynamic_cli::context::downcast_ref::<FileManagerContext>(context)
            .ok_or_else(|| {
                DynamicCliError::Execution(dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "FileManagerContext".to_string(),
                })
            })?;

        println!("\nFile Manager Statistics");
        println!("═════════════════════════════════════");
        println!("Files processed: {}", ctx.files_processed);
        
        if let Some(ref dir) = ctx.current_dir {
            println!("Current directory: {}", dir.display());
        }
        
        if let Some(ref op) = ctx.last_operation {
            println!("Last operation: {}", op);
        }

        Ok(())
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> Result<()> {
    let config_path = "examples/configs/file_manager.yaml";

    CliBuilder::new()
        .config_file(config_path)
        .context(Box::new(FileManagerContext::default()))
        .register_handler("list_handler", Box::new(ListCommand))
        .register_handler("info_handler", Box::new(InfoCommand))
        .register_handler("search_handler", Box::new(SearchCommand))
        .register_handler("stats_handler", Box::new(StatsCommand))
        .build()?
        .run()
}
