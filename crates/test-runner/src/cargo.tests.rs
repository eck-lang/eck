use super::*;

/// Forwards a successful cargo invocation.
#[test]
fn reports_successful_cargo_invocation() {
    assert_eq!(
        run_cargo(&project_root(), ["--version"], CargoOutput::Capture),
        Ok(true)
    );
}

/// Reports a failing cargo invocation without treating it as a runner error.
#[test]
fn reports_failing_cargo_invocation() {
    assert_eq!(
        run_cargo(
            &project_root(),
            ["nonexistent-subcommand-for-tests"],
            CargoOutput::Capture
        ),
        Ok(false)
    );
}
