// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::config::{AndromedaConfig, ConfigManager};
use crate::error::{Result, read_file_with_context};
use andromeda_core::{
    HostData, ImportMap, Runtime, RuntimeConfig, RuntimeFile,
};
use andromeda_runtime::{
    recommended_builtins, recommended_eventloop_handler, recommended_extensions,
};
use console::Style;
use nova_vm::ecmascript::{
    scripts_and_modules::script::{parse_script, script_evaluation},
    types::String as NovaString,
};
use nova_vm::engine::context::Bindable;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Test result structure
#[derive(Debug, serde::Deserialize)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub error: Option<String>,
    pub duration: u128,
}

/// Run tests
pub fn run_tests(paths: Vec<PathBuf>, verbose: bool, _watch: bool) -> Result<()> {
    println!("DEBUG: Current working directory: {:?}", std::env::current_dir());
    // Load configuration
    let config = ConfigManager::load_or_default(None);

    // Find test files
    let test_files = find_test_files(&paths, &config)?;

    if test_files.is_empty() {
        let warning = Style::new().yellow().bold().apply_to("⚠️");
        let msg = Style::new()
            .yellow()
            .apply_to("No test files found.");
        println!("{warning} {msg}");
        return Ok(());
    }

    let count = Style::new().cyan().apply_to(test_files.len());
    println!("Found {count} test file(s) to run");
    println!("{}", Style::new().dim().apply_to("─".repeat(40)));

    let mut total_tests = 0;
    let mut passed_tests = 0;
    let mut failed_tests = 0;
    let mut total_duration = 0u128;

    for test_file in &test_files {
        match run_single_test_file(test_file, verbose) {
            Ok(results) => {
                let (passed, failed, duration) = print_test_results(test_file, &results, verbose);
                total_tests += results.len();
                passed_tests += passed;
                failed_tests += failed;
                total_duration += duration;
            }
            Err(e) => {
                println!("❌ Failed to run test file {}: {}", test_file.display(), e);
                failed_tests += 1;
            }
        }
    }

    println!();
    println!("{}", Style::new().dim().apply_to("─".repeat(40)));
    let success = if failed_tests == 0 {
        Style::new().green().bold().apply_to("✅")
    } else {
        Style::new().red().bold().apply_to("❌")
    };
    let summary = Style::new().white().bold().apply_to("Test Summary");
    println!("{success} {summary}:");

    let passed_style = Style::new().green().bold();
    let failed_style = Style::new().red().bold();
    let total_style = Style::new().cyan().bold();

    println!("   {} {} passed", passed_style.apply_to("✓"), passed_tests);
    if failed_tests > 0 {
        println!("   {} {} failed", failed_style.apply_to("✗"), failed_tests);
    }
    println!("   {} {} total", total_style.apply_to("Σ"), total_tests);

    if total_tests > 0 {
        let duration_ms = total_duration / 1000;
        let duration_style = Style::new().dim();
        println!("   {} {}ms", duration_style.apply_to("⏱️"), duration_ms);
    }

    if failed_tests > 0 {
        Err(crate::error::AndromedaError::runtime_error(
            format!("{} test(s) failed", failed_tests),
            None,
            None,
            None,
            None,
        ))
    } else {
        Ok(())
    }
}

fn find_test_files(paths: &[PathBuf], _config: &AndromedaConfig) -> Result<Vec<PathBuf>> {
    let mut test_files = Vec::new();

    for path in paths {
            if is_test_file(path) {
                test_files.push(path.clone());
            } else if path.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                let entry_path = entry.path();
                if entry_path.is_file() && is_test_file(entry_path) {
                    test_files.push(entry_path.to_path_buf());
                }
            }
        }
    }

    // If no paths specified, search current directory
    if paths.is_empty() {
        for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            let entry_path = entry.path();
            if entry_path.is_file() && is_test_file(entry_path) {
                test_files.push(entry_path.to_path_buf());
            }
        }
    }

    Ok(test_files)
}

fn is_test_file(path: &Path) -> bool {
    let file_name = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    // Check for common test file patterns
    file_name.ends_with(".test.ts") ||
    file_name.ends_with(".test.js") ||
    file_name.ends_with(".spec.ts") ||
    file_name.ends_with(".spec.js") ||
    file_name == "__tests__" ||
    file_name.contains(".test.") ||
    file_name.contains(".spec.")
}

fn run_single_test_file(test_file: &Path, verbose: bool) -> Result<Vec<TestResult>> {
    // Read the test file content
    let content = read_file_with_context(test_file)?;

    // Wrap the test file content with result collection
    let wrapped_content = format!(
        r#"
// Reset test state at the start
globalThis.__andromeda_test_reset();

// Execute the original test file
{}

// Test execution completed
"#,
        content
    );

    let content_bytes = wrapped_content.into_bytes();
    let content_ref = Box::leak(content_bytes.into_boxed_slice());

    let runtime_file = RuntimeFile::Embedded {
        path: test_file.to_string_lossy().to_string(),
        content: content_ref,
    };

    let config = ConfigManager::load_or_default(Some(test_file.parent().unwrap_or(Path::new("."))));

    let import_map = ImportMap::default(); // Simplified

    let (macro_task_tx, macro_task_rx) = std::sync::mpsc::channel();
    let host_data = HostData::new(macro_task_tx);

    let runtime = Runtime::new(
        RuntimeConfig {
            no_strict: config.runtime.no_strict,
            files: vec![runtime_file],
            verbose,
            extensions: recommended_extensions(),
            builtins: recommended_builtins(),
            eventloop_handler: recommended_eventloop_handler,
            macro_task_rx,
            import_map: Some(import_map),
        },
        host_data,
    );

    let mut runtime_output = runtime.run();

    match runtime_output.result {
        Ok(_) => {
            // Extract test results from the runtime by executing JavaScript code
            let results = runtime_output.agent.run_in_realm(&runtime_output.realm_root, |agent, mut gc| {
                // Parse and execute JavaScript code to call the global function
                let code = "__andromeda_test_get_results()";
                let realm = agent.current_realm(gc.nogc());
                let source_text = NovaString::from_str(agent, code, gc.nogc());
                let script = match parse_script(
                    agent,
                    source_text,
                    realm,
                    true, // strict mode
                    None,
                    gc.nogc(),
                ) {
                    Ok(script) => script,
                    Err(_) => return vec![],
                };
                let eval_result = script_evaluation(agent, script.unbind(), gc.reborrow()).unbind();
                match eval_result {
                    Ok(value) => {
                        match value.to_string(agent, gc.reborrow()) {
                            Ok(result_str) => {
                                match serde_json::from_str::<Vec<TestResult>>(result_str.as_str(agent).expect("String is not valid UTF-8")) {
                                    Ok(results) => results,
                                    Err(_) => vec![],
                                }
                            }
                            Err(_) => vec![],
                        }
                    }
                    Err(_) => vec![],
                }
            });
            Ok(results)
        }
        Err(error) => {
            let error_message = runtime_output
                .agent
                .run_in_realm(&runtime_output.realm_root, |agent, gc| {
                    error
                        .value()
                        .string_repr(agent, gc)
                        .as_str(agent)
                        .expect("String is not valid UTF-8")
                        .to_string()
                });

            Err(crate::error::AndromedaError::runtime_error(
                format!("Test execution failed: {}", error_message),
                Some(test_file.to_string_lossy().to_string()),
                None,
                None,
                None,
            ))
        }
    }
}

fn print_test_results(test_file: &Path, results: &[TestResult], verbose: bool) -> (usize, usize, u128) {
    let mut passed = 0;
    let mut failed = 0;
    let mut total_duration = 0u128;

    let file_name = Style::new().cyan().bold().apply_to(test_file.display());
    println!("Running tests in {file_name}:");

    for result in results {
        total_duration += result.duration;
        if result.passed {
            passed += 1;
            let check = Style::new().green().apply_to("✓");
            let name = Style::new().white().apply_to(&result.name);
            let duration = if verbose {
                format!(" ({}μs)", result.duration)
            } else {
                String::new()
            };
            println!("  {check} {name}{duration}");
        } else {
            failed += 1;
            let cross = Style::new().red().apply_to("✗");
            let name = Style::new().white().apply_to(&result.name);
            println!("  {cross} {name}");
            if let Some(error) = &result.error {
                let error_msg = Style::new().red().dim().apply_to(error);
                println!("    {error_msg}");
            }
        }
    }

    (passed, failed, total_duration)
}