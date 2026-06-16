pub struct Vendor {
    pub origin: &'static str,
    pub sync_enabled: bool,
    pub gitea_label_ids: &'static [i64],
}

const VENDORS: &[Vendor] = &[
    Vendor {
        origin: "square",
        sync_enabled: true,
        gitea_label_ids: &[1307],
    },
    Vendor {
        origin: "coinos",
        sync_enabled: true,
        gitea_label_ids: &[],
    },
    Vendor {
        origin: "btcpayserver",
        sync_enabled: true,
        gitea_label_ids: &[1538],
    },
    Vendor {
        origin: "square-test",
        sync_enabled: true,
        gitea_label_ids: &[1551],
    },
    Vendor {
        origin: "bitcoin-jungle",
        sync_enabled: true,
        gitea_label_ids: &[1552],
    },
];

pub fn get(origin: &str) -> Option<&'static Vendor> {
    VENDORS.iter().find(|vendor| vendor.origin == origin)
}
