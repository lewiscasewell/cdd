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
        .stdout(predicate::str::contains("numberOfCycles"));
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
