pub struct Vendor {
    pub origin: &'static str,
    pub sync_enabled: bool,
    pub payment_provider: Option<&'static str>,
    pub payment_tag_name: Option<&'static str>,
    pub payment_tag_value: Option<&'static str>,
    pub gitea_label_ids: &'static [i64],
}

const VENDORS: &[Vendor] = &[
    Vendor {
        origin: "square",
        sync_enabled: true,
        payment_provider: Some("square"),
        payment_tag_name: Some("payment:lightning:operator"),
        payment_tag_value: Some("square"),
        gitea_label_ids: &[1307],
    },
    Vendor {
        origin: "coinos",
        sync_enabled: true,
        payment_provider: Some("coinos"),
        payment_tag_name: Some("payment:coinos"),
        payment_tag_value: Some("yes"),
        gitea_label_ids: &[],
    },
    Vendor {
        origin: "btcpayserver",
        sync_enabled: true,
        payment_provider: Some("btcpayserver"),
        payment_tag_name: Some("payment:btcpayserver"),
        payment_tag_value: Some("yes"),
        gitea_label_ids: &[1538],
    },
];

pub fn get(origin: &str) -> Option<&'static Vendor> {
    VENDORS.iter().find(|vendor| vendor.origin == origin)
}

pub fn origin_for_payment_tag(tag_name: &str, tag_value: &str) -> Option<&'static str> {
    VENDORS.iter().find_map(|vendor| {
        if vendor.payment_tag_name == Some(tag_name) && vendor.payment_tag_value == Some(tag_value)
        {
            Some(vendor.origin)
        } else {
            None
        }
    })
}
