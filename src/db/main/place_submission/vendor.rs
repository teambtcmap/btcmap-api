pub struct Vendor {
    pub origin: &'static str,
    pub sync_enabled: bool,
    pub payment_provider: Option<&'static str>,
    pub payment_tag_name: Option<&'static str>,
    pub payment_tag_value: Option<&'static str>,
}

const VENDORS: &[Vendor] = &[
    Vendor {
        origin: "square",
        sync_enabled: true,
        payment_provider: Some("square"),
        payment_tag_name: Some("payment:lightning:operator"),
        payment_tag_value: Some("square"),
    },
    Vendor {
        origin: "coinos",
        sync_enabled: true,
        payment_provider: Some("coinos"),
        payment_tag_name: Some("payment:coinos"),
        payment_tag_value: Some("yes"),
    },
];

pub fn get(origin: &str) -> Option<&'static Vendor> {
    VENDORS.iter().find(|vendor| vendor.origin == origin)
}

pub fn sync_enabled(origin: &str) -> bool {
    get(origin).is_some_and(|vendor| vendor.sync_enabled)
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
