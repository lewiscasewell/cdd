//! Watch mode for file change detection.
//!
//! This module is only available when the `watch` feature is enabled.

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

/// Runs analysis in watch mode, re-running on file changes.
pub fn watch_and_run<F>(dir: &Path, excludes: &[String], run_analysis: F) -> anyhow::Result<()>
where
    F: Fn(),
{
    // Run initial analysis
    clear_terminal();
    run_analysis();

    // Set up file watcher with debouncing
    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;

    debouncer.watcher().watch(dir, RecursiveMode::Recursive)?;

    log::info!("Watching for changes in {}...", dir.display());
    log::info!("Press Ctrl+C to stop.\n");

    // Watch for file changes
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                // Check if any relevant files changed
                let relevant_change = events.iter().any(|event| {
                    if event.kind != DebouncedEventKind::Any {
                        return false;
                    }

                    let path = &event.path;

                    // Check if file has a relevant extension
                    let is_relevant_ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| matches!(ext, "ts" | "tsx" | "js" | "jsx" | "cjs" | "mjs"))
                        .unwrap_or(false);

                    if !is_relevant_ext {
                        return false;
                    }

                    // Check if file is in an excluded directory
                    let in_excluded = excludes.iter().any(|exclude| {
                        path.components().any(|c| {
                            c.as_os_str()
                                .to_str()
                                .map(|s| s == exclude)
                                .unwrap_or(false)
                        })
                    });

                    !in_excluded
                });

                if relevant_change {
                    clear_terminal();
                    log::info!("File change detected, re-running analysis...\n");
                    run_analysis();
                    log::info!("\nWatching for changes...");
                }
            }
            Ok(Err(errors)) => {
                log::error!("Watch error: {:?}", errors);
            }
            Err(e) => {
                log::error!("Watch channel error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

fn clear_terminal() {
    // ANSI escape code to clear screen and move cursor to top-left
    print!("\x1B[2J\x1B[1;1H");
}
