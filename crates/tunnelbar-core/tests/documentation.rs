//! Keeps the documented configuration examples in sync with the actual format.

use tunnelbar_core::model::{Config, Forward};
use tunnelbar_core::validation::validate;

#[test]
fn documented_example_is_a_valid_configuration() {
    let config = Config::parse(include_str!("../../../docs/examples/tunnels.toml")).unwrap();
    assert_eq!(validate(&config), Ok(()));

    let kinds: Vec<_> = config
        .tunnels
        .iter()
        .flat_map(|tunnel| &tunnel.forwards)
        .collect();
    assert!(
        kinds
            .iter()
            .any(|forward| matches!(forward, Forward::Local { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|forward| matches!(forward, Forward::Remote { .. }))
    );
    assert!(
        kinds
            .iter()
            .any(|forward| matches!(forward, Forward::Dynamic { .. }))
    );
}
