//! CLI fuzzing tests using figue's arbitrary helper assertions.

use teamy_youtube::cli::Cli;

#[test]
fn fuzz_cli_args_consistency() {
    if let Err(error) =
        figue::assert_to_args_consistency::<Cli>(figue::TestToArgsConsistencyConfig::default())
    {
        panic!("CLI argument consistency check failed:\n{error}");
    }
}

#[test]
fn fuzz_cli_args_roundtrip() {
    if let Err(error) =
        figue::assert_to_args_roundtrip::<Cli>(figue::TestToArgsRoundTrip::default())
    {
        panic!("CLI argument roundtrip check failed:\n{error}");
    }
}

#[test]
fn takeout_optional_archive_defaults_roundtrip_regression() {
    // Previously failed before invocation when all flattened archive selectors were omitted.
    figue::assert_to_args_roundtrip::<Cli>(figue::TestToArgsRoundTrip {
        root_seed: Some(6_582_314_645_249_353_255),
        ..Default::default()
    })
    .expect("Takeout Inspect/Read optional archive defaults should roundtrip");
}

#[test]
fn takeout_empty_selectors_parse_with_defaults() {
    for subcommand in ["inspect", "read"] {
        let config = figue::builder::<Cli>()
            .unwrap()
            .cli(|cli| cli.args(["takeout", subcommand]).strict())
            .build();
        let parsed = figue::Driver::new(config)
            .run()
            .into_result()
            .unwrap()
            .value;
        let teamy_youtube::cli::Command::Takeout(args) = parsed.command else {
            panic!("expected Takeout command")
        };
        let archive = match args.command {
            teamy_youtube::cli::takeout::TakeoutCommand::Inspect(args) => args.archive,
            teamy_youtube::cli::takeout::TakeoutCommand::Read(args) => args.archive,
        };
        assert_eq!(
            archive,
            teamy_youtube::cli::takeout::TakeoutZipArgs::default()
        );
    }
}
