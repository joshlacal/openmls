use std::time::{SystemTime, UNIX_EPOCH};

use openmls::{
    prelude::{test_utils::new_credential, *},
    treesync::errors::{LeafNodeValidationError, LifetimeError},
};
use openmls_basic_credential::SignatureKeyPair;
use openmls_test::openmls_test;

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn key_package<Provider: OpenMlsProvider>(
    provider: &Provider,
    signer_name: &[u8],
    ciphersuite: Ciphersuite,
    lifetime: Lifetime,
) -> KeyPackage {
    let (credential, signer) =
        new_credential(provider, signer_name, ciphersuite.signature_algorithm());
    KeyPackage::builder()
        .key_package_lifetime(lifetime)
        .build(ciphersuite, provider, &signer, credential)
        .unwrap()
        .key_package()
        .clone()
}

fn build_add<Provider: OpenMlsProvider>(
    group: &mut MlsGroup,
    provider: &Provider,
    signer: &SignatureKeyPair,
    key_package: KeyPackage,
    lifetime_policy: LeafNodeLifetimePolicy,
) -> Result<(), CreateCommitError> {
    group
        .commit_builder()
        .propose_adds([key_package])
        .leaf_node_lifetime_policy(lifetime_policy)
        .load_psks(provider.storage())
        .unwrap()
        .build(provider.rand(), provider.crypto(), signer, |_| true)
        .map(|_| ())
}

fn stage_add<Provider: OpenMlsProvider>(
    group: &mut MlsGroup,
    provider: &Provider,
    signer: &SignatureKeyPair,
    key_package: KeyPackage,
    lifetime_policy: LeafNodeLifetimePolicy,
) -> Result<MlsMessageOut, CreateCommitError> {
    let bundle = group
        .commit_builder()
        .propose_adds([key_package])
        .leaf_node_lifetime_policy(lifetime_policy)
        .load_psks(provider.storage())
        .unwrap()
        .build(provider.rand(), provider.crypto(), signer, |_| true)?
        .stage_commit(provider)
        .unwrap();
    let commit = bundle.commit().clone();
    group.merge_pending_commit(provider).unwrap();
    Ok(commit)
}

fn protocol_message(message: &MlsMessageOut) -> ProtocolMessage {
    MlsMessageIn::from(message.clone())
        .try_into_protocol_message()
        .unwrap()
}

fn lifetime_error<StorageError>(error: &ProcessMessageError<StorageError>) -> Option<&LifetimeError> {
    match error {
        ProcessMessageError::ValidationError(ValidationError::KeyPackageVerifyError(
            KeyPackageVerifyError::LifetimeError(error),
        )) => Some(error),
        _ => None,
    }
}

#[openmls_test]
fn commit_builder_uses_supplied_time_for_add_lifetimes() {
    let provider = Provider::default();
    let (credential, signer) =
        new_credential(&provider, b"Alice", ciphersuite.signature_algorithm());
    let config = MlsGroupCreateConfig::builder()
        .ciphersuite(ciphersuite)
        .build();
    let mut group = MlsGroup::new(&provider, &signer, &config, credential).unwrap();
    let wall_now = now();

    let future = key_package(
        &Provider::default(),
        b"Future",
        ciphersuite,
        Lifetime::init(wall_now + 1_000, wall_now + 2_000),
    );
    let error = build_add(
        &mut group,
        &provider,
        &signer,
        future.clone(),
        LeafNodeLifetimePolicy::Verify,
    )
    .expect_err("the default policy must use the system clock");
    assert!(matches!(
        error,
        CreateCommitError::ProposalValidationError(ProposalValidationError::LeafNodeValidation(
            LeafNodeValidationError::Lifetime(LifetimeError::NotValidYet { .. })
        ))
    ));
    build_add(
        &mut group,
        &provider,
        &signer,
        future,
        LeafNodeLifetimePolicy::VerifyAt(wall_now + 1_500),
    )
    .expect("the supplied time is inside the key package lifetime");

    let current = key_package(
        &Provider::default(),
        b"Current",
        ciphersuite,
        Lifetime::init(wall_now.saturating_sub(60), wall_now + 1_000),
    );
    let error = build_add(
        &mut group,
        &provider,
        &signer,
        current,
        LeafNodeLifetimePolicy::VerifyAt(wall_now + 1_500),
    )
    .expect_err("the supplied time is after the key package lifetime");
    assert!(matches!(
        error,
        CreateCommitError::ProposalValidationError(ProposalValidationError::LeafNodeValidation(
            LeafNodeValidationError::Lifetime(LifetimeError::Expired { .. })
        ))
    ));
}

#[openmls_test]
fn existing_member_processing_uses_supplied_time_for_add_lifetimes() {
    let wall_now = now();
    let config = MlsGroupCreateConfig::builder()
        .ciphersuite(ciphersuite)
        .wire_format_policy(PURE_PLAINTEXT_WIRE_FORMAT_POLICY)
        .use_ratchet_tree_extension(true)
        .build();

    let alice_provider = Provider::default();
    let (alice_credential, alice_signer) = new_credential(
        &alice_provider,
        b"Alice",
        ciphersuite.signature_algorithm(),
    );
    let mut alice_group =
        MlsGroup::new(&alice_provider, &alice_signer, &config, alice_credential).unwrap();

    let charlie_provider = Provider::default();
    let (charlie_credential, charlie_signer) = new_credential(
        &charlie_provider,
        b"Charlie",
        ciphersuite.signature_algorithm(),
    );
    let charlie_key_package = KeyPackage::builder()
        .key_package_lifetime(Lifetime::init(
            wall_now.saturating_sub(60),
            wall_now + 3_600,
        ))
        .build(
            ciphersuite,
            &charlie_provider,
            &charlie_signer,
            charlie_credential,
        )
        .unwrap()
        .key_package()
        .clone();
    let (_, welcome, _) = alice_group
        .add_members(&alice_provider, &alice_signer, &[charlie_key_package])
        .unwrap();
    alice_group
        .merge_pending_commit(&alice_provider)
        .unwrap();
    let welcome = match MlsMessageIn::from(welcome).extract() {
        MlsMessageBodyIn::Welcome(welcome) => welcome,
        _ => panic!("expected a Welcome"),
    };
    let mut charlie_group =
        StagedWelcome::new_from_welcome(&charlie_provider, config.join_config(), welcome, None)
            .unwrap()
            .into_group(&charlie_provider)
            .unwrap();

    let future = key_package(
        &Provider::default(),
        b"Future",
        ciphersuite,
        Lifetime::init(wall_now + 1_000, wall_now + 2_000),
    );
    let error = build_add(
        &mut alice_group,
        &alice_provider,
        &alice_signer,
        future.clone(),
        LeafNodeLifetimePolicy::Verify,
    )
    .expect_err("the sender default must use the system clock");
    assert!(matches!(
        error,
        CreateCommitError::ProposalValidationError(ProposalValidationError::LeafNodeValidation(
            LeafNodeValidationError::Lifetime(LifetimeError::NotValidYet { .. })
        ))
    ));

    let supplied_now = wall_now + 1_500;
    let commit = stage_add(
        &mut alice_group,
        &alice_provider,
        &alice_signer,
        future,
        LeafNodeLifetimePolicy::VerifyAt(supplied_now),
    )
    .unwrap();
    let error = charlie_group
        .process_message(&charlie_provider, protocol_message(&commit))
        .expect_err("the receiver default must use the system clock");
    assert!(matches!(
        lifetime_error(&error),
        Some(LifetimeError::NotValidYet { .. })
    ));
    let processed = charlie_group
        .process_message_with_lifetime_policy(
            &charlie_provider,
            protocol_message(&commit),
            LeafNodeLifetimePolicy::VerifyAt(supplied_now),
        )
        .unwrap();
    let ProcessedMessageContent::StagedCommitMessage(staged) = processed.into_content() else {
        panic!("expected a staged Commit");
    };
    charlie_group
        .merge_staged_commit(&charlie_provider, *staged)
        .unwrap();
    assert_eq!(alice_group.epoch(), charlie_group.epoch());

    let historical_now = wall_now.saturating_sub(1_500);
    let expired = key_package(
        &Provider::default(),
        b"Expired",
        ciphersuite,
        Lifetime::init(
            wall_now.saturating_sub(2_000),
            wall_now.saturating_sub(1_000),
        ),
    );
    let expired_commit = stage_add(
        &mut alice_group,
        &alice_provider,
        &alice_signer,
        expired,
        LeafNodeLifetimePolicy::VerifyAt(historical_now),
    )
    .unwrap();
    let error = charlie_group
        .process_message_with_lifetime_policy(
            &charlie_provider,
            protocol_message(&expired_commit),
            LeafNodeLifetimePolicy::VerifyAt(wall_now),
        )
        .expect_err("the receiver supplied time is after the key package lifetime");
    assert!(matches!(
        lifetime_error(&error),
        Some(LifetimeError::Expired { .. })
    ));
}
