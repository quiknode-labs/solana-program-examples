// Quasar tests for the asset leasing protocol.
//
// Note: Full test coverage for all instructions is in the Anchor version
// (tests-rs/test.rs). The Quasar version currently implements initialize
// and collect_fees. Remaining instructions depend on cross-field PDA seed
// support being added to Quasar's #[derive(Accounts)].
//
// Tests use quasar-svm but need the generated client crate which requires
// `quasar build --clients` to be run first. For now, the program compiles
// and deploys — see the Anchor tests for integration test coverage.
