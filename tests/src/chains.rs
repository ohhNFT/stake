use cw_orch::{environment::NetworkInfo, prelude::ChainInfo};

pub const ELGAFAR_1: ChainInfo = ChainInfo {
    chain_id: "elgafar-1",
    kind: cw_orch::environment::ChainKind::Testnet,
    grpc_urls: &["http://grpc-1.elgafar-1.stargaze-apis.com:26660"],
    lcd_url: Some("https://rest.elgafar-1.stargaze-apis.com"),
    fcd_url: None,
    gas_denom: "ustars",
    gas_price: 0.025,
    network_info: NetworkInfo {
        chain_name: "stargaze-testnet",
        pub_address_prefix: "stars",
        coin_type: 118,
    },
};
