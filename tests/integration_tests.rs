use assert_cmd::Command;
use predicates::prelude::*;

fn cdd() -> Command {
    Command::cargo_bin("cdd").unwrap()
}

#[test]
fn test_example_monorepo_detects_all_cycles() {
    // Exclude dist to only scan source files
    cdd()
        .args([
            "-n",
            "5",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Found 5 circular dependencies"));
}

#[test]
fn test_example_monorepo_fails_with_wrong_expected_count() {
    cdd()
        .args([
            "-n",
            "0",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Expected 0 cycle(s), but found 5"));
}

#[test]
fn test_detects_api_service_cycle() {
    cdd()
        .args(["./fixtures/example-monorepo/packages/api"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("orderService.ts"))
        .stderr(predicate::str::contains("userService.ts"));
}

#[test]
fn test_detects_web_component_cycle() {
    cdd()
        .args(["./fixtures/example-monorepo/packages/web"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Button.tsx"))
        .stderr(predicate::str::contains("Modal.tsx"))
        .stderr(predicate::str::contains("Form.tsx"));
}

#[test]
fn test_detects_hook_cycle() {
    cdd()
        .args(["./fixtures/example-monorepo/packages/web"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("useAuth.ts"))
        .stderr(predicate::str::contains("useUser.ts"));
}

#[test]
fn test_detects_shared_utils_cycle() {
    cdd()
        .args(["./fixtures/example-monorepo/packages/shared"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("stringUtils.ts"))
        .stderr(predicate::str::contains("arrayUtils.ts"))
        .stderr(predicate::str::contains("objectUtils.ts"));
}

#[test]
fn test_exclude_flag_works() {
    // Excluding the services directory should remove the API cycle
    cdd()
        .args([
            "--exclude",
            "services",
            "./fixtures/example-monorepo/packages/api",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("no circular dependencies found"));
}

#[test]
fn test_silent_flag_suppresses_output() {
    cdd()
        .args(["--silent", "./fixtures/example-monorepo/packages"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_empty_directory_no_cycles() {
    cdd()
        .args(["./fixtures/example-monorepo/packages/api/src/utils"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no circular dependencies found"));
}

#[test]
fn test_version_flag() {
    cdd()
        .args(["--version"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Circular Dependency Detector"));
}

#[test]
fn test_help_flag() {
    cdd()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("exclude"))
        .stdout(predicate::str::contains("numberOfCycles"))
        .stdout(predicate::str::contains("tsconfig"));
}

#[test]
fn test_specific_cycle_count_api_only() {
    // API package should have exactly 1 cycle (in source)
    cdd()
        .args([
            "-n",
            "1",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages/api",
        ])
        .assert()
        .success();
}

#[test]
fn test_specific_cycle_count_web_only() {
    // Web package should have exactly 2 cycles (hooks + components) in source
    cdd()
        .args([
            "-n",
            "2",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages/web",
        ])
        .assert()
        .success();
}

#[test]
fn test_specific_cycle_count_shared_only() {
    // Shared package should have exactly 2 cycles (utils + type-only) in source
    cdd()
        .args([
            "-n",
            "2",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages/shared",
        ])
        .assert()
        .success();
}

// ============ Type-only import tests ============

#[test]
fn test_type_only_cycle_detected_by_default() {
    // Without flag, type-only cycles should be detected
    cdd()
        .args([
            "-n",
            "1",
            "./fixtures/example-monorepo/packages/shared/src/type-only",
        ])
        .assert()
        .success();
}

#[test]
fn test_type_only_cycle_ignored_with_flag() {
    // With --ignore-type-imports, type-only cycles should be ignored
    cdd()
        .args([
            "--ignore-type-imports",
            "-n",
            "0",
            "./fixtures/example-monorepo/packages/shared/src/type-only",
        ])
        .assert()
        .success();
}

#[test]
fn test_ignore_type_imports_reduces_cycle_count() {
    // Full monorepo source: 5 cycles without flag, 4 with flag
    cdd()
        .args([
            "-n",
            "5",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages",
        ])
        .assert()
        .success();

    cdd()
        .args([
            "--ignore-type-imports",
            "-n",
            "4",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages",
        ])
        .assert()
        .success();
}

// ============ Built output (dist) tests ============

#[test]
fn test_dist_has_fewer_cycles_than_source() {
    // Built JS has fewer cycles because type-only imports are erased
    // Source: 5 cycles, Dist: 3 cycles
    cdd()
        .args([
            "-n",
            "3",
            "--exclude",
            "src",
            "./fixtures/example-monorepo/packages",
        ])
        .assert()
        .success();
}

// ============ CommonJS support tests ============

#[test]
fn test_commonjs_require_cycle_detected() {
    // CommonJS require() cycles should be detected
    cdd()
        .args(["-n", "1", "./fixtures/commonjs-build"])
        .assert()
        .success()
        .stderr(predicate::str::contains("serviceA.js"))
        .stderr(predicate::str::contains("serviceB.js"));
}

// ============ Config file tests ============
// Note: Config file tests use dedicated fixtures to avoid parallel test interference.

#[test]
fn test_config_file_is_loaded() {
    // Use dedicated config-test-a fixture (empty directory)
    let config_path = "./fixtures/config-test-a/.cddrc.json";

    // Clean up any leftover config file first
    let _ = std::fs::remove_file(config_path);

    // Create a config file expecting 0 cycles (no files = no cycles)
    std::fs::write(config_path, r#"{"expected_cycles": 0}"#).unwrap();

    // Should succeed with config file settings
    let result = cdd()
        .args(["./fixtures/config-test-a"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no circular dependencies found"));

    // Clean up
    let _ = std::fs::remove_file(config_path);

    result.stderr(predicate::str::contains("Expected 0 cycle(s) and found 0"));
}

#[test]
fn test_cli_overrides_config_file() {
    // Use dedicated config-test-b fixture
    let config_path = "./fixtures/config-test-b/.cddrc.json";

    // Clean up any leftover config file first
    let _ = std::fs::remove_file(config_path);

    // Create a config file expecting 99 cycles (wrong)
    std::fs::write(config_path, r#"{"expected_cycles": 99}"#).unwrap();

    // CLI -n should override config file's expected_cycles
    let result = cdd()
        .args(["-n", "0", "./fixtures/config-test-b"])
        .assert()
        .success();

    // Clean up
    let _ = std::fs::remove_file(config_path);

    result.stderr(predicate::str::contains("Expected 0 cycle(s) and found 0"));
}

// ============ Timing output tests ============

#[test]
fn test_analysis_timing_is_shown() {
    // Use -n 0 and a directory with no cycles to ensure success
    cdd()
        .args([
            "-n",
            "0",
            "./fixtures/example-monorepo/packages/api/src/utils",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Analysis completed in"));
}

// ============ Tsconfig flag tests ============

#[test]
fn test_tsconfig_flag_is_accepted() {
    // Should accept --tsconfig flag without error
    cdd()
        .args([
            "--tsconfig",
            "./fixtures/example-monorepo/packages/api/tsconfig.json",
            "-n",
            "1",
            "--exclude",
            "dist",
            "./fixtures/example-monorepo/packages/api",
        ])
        .assert()
        .success();
}

// ============ Watch flag tests ============

#[test]
#[cfg(feature = "watch")]
fn test_watch_flag_in_help() {
    cdd()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--watch"))
        .stdout(predicate::str::contains("Watch mode"));
}

// ============ Workspace tests ============

#[test]
fn test_workspace_flag_in_help() {
    // Workspaces are now auto-detected, --no-workspace disables
    cdd()
        .args(["--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--no-workspace"))
        .stdout(predicate::str::contains("monorepo"));
}

#[test]
fn test_workspace_detects_packages() {
    // Workspaces are auto-detected, should show package count
    cdd()
        .args(["-n", "1", "./fixtures/workspace-monorepo"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "Detected workspace with 3 packages",
        ));
}

#[test]
fn test_workspace_detects_cross_package_cycles() {
    // Should detect circular dependency between @test/ui -> @test/utils -> @test/core -> @test/ui
    cdd()
        .args(["-n", "1", "./fixtures/workspace-monorepo"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Found 1 circular dependencies"));
}

#[test]
fn test_workspace_disabled_with_flag() {
    // With --no-workspace, bare package imports are not resolved
    // So no cycles would be detected (only relative imports are resolved)
    cdd()
        .args(["--no-workspace", "-n", "0", "./fixtures/workspace-monorepo"])
        .assert()
        .success()
        .stderr(predicate::str::contains("no circular dependencies found"));
}

#[test]
fn test_workspace_shows_package_names() {
    // Should show the detected package names (auto-detected)
    cdd()
        .args(["-n", "1", "./fixtures/workspace-monorepo"])
        .assert()
        .success()
        .stderr(predicate::str::contains("@test/ui"))
        .stderr(predicate::str::contains("@test/utils"))
        .stderr(predicate::str::contains("@test/core"));
}

// ============ Complex workspace tests ============
// Workspaces are now auto-detected, no --workspace flag needed

#[test]
fn test_complex_workspace_detects_all_cycles() {
    // Complex workspace with deep nesting, subpath imports, and multiple cycle types
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Found 4 circular dependencies"));
}

#[test]
fn test_complex_workspace_detects_internal_store_cycle() {
    // authStore <-> userStore cycle within data-layer
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("authStore.ts"))
        .stderr(predicate::str::contains("userStore.ts"));
}

#[test]
fn test_complex_workspace_detects_internal_hook_cycle() {
    // useAuth <-> useUser cycle within data-layer
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("useAuth.ts"))
        .stderr(predicate::str::contains("useUser.ts"));
}

#[test]
fn test_complex_workspace_detects_component_cycle() {
    // buttons -> forms -> modals cycle within design-system
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("buttons/index.ts"))
        .stderr(predicate::str::contains("forms/index.ts"))
        .stderr(predicate::str::contains("modals/index.ts"));
}

#[test]
fn test_complex_workspace_detects_cross_package_cycle() {
    // modalStore -> shared cycle via subpath imports
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("modalStore.ts"))
        .stderr(predicate::str::contains("shared/src/index.ts"));
}

#[test]
fn test_complex_workspace_resolves_subpath_imports() {
    // Tests that @acme/data-layer/stores/modalStore resolves correctly
    // (this import is in design-system/modals)
    cdd()
        .args(["-n", "4", "./fixtures/workspace-complex"])
        .assert()
        .success()
        .stderr(predicate::str::contains("22 files")); // Should collect all 22 files
}
