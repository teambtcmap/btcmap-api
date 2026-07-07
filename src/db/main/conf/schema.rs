use rusqlite::Row;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "conf";

pub enum Columns {
    PaywallAddElementCommentPriceSat,
    PaywallBoostElement30DaysPriceSat,
    PaywallBoostElement90DaysPriceSat,
    PaywallBoostElement365DaysPriceSat,
    LNbitsInvoiceKey,
    GiteaApiKey,
    MatrixBotPassword,
    LndInvoicesMacaroon,
    LndReadonlyMacaroon,
    PpqKey,
    XpubSpending,
    XpubDonations,
    XpubTreasury,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::PaywallAddElementCommentPriceSat => "paywall_add_element_comment_price_sat",
            Columns::PaywallBoostElement30DaysPriceSat => "paywall_boost_element_30d_price_sat",
            Columns::PaywallBoostElement90DaysPriceSat => "paywall_boost_element_90d_price_sat",
            Columns::PaywallBoostElement365DaysPriceSat => "paywall_boost_element_365d_price_sat",
            Columns::LNbitsInvoiceKey => "lnbits_invoice_key",
            Columns::GiteaApiKey => "gitea_api_key",
            Columns::MatrixBotPassword => "matrix_bot_password",
            Columns::LndInvoicesMacaroon => "lnd_invoices_macaroon",
            Columns::LndReadonlyMacaroon => "lnd_readonly_macaroon",
            Columns::PpqKey => "ppq_key",
            Columns::XpubSpending => "xpub_spending",
            Columns::XpubDonations => "xpub_donations",
            Columns::XpubTreasury => "xpub_treasury",
        }
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Conf {
    pub paywall_add_element_comment_price_sat: i64,
    pub paywall_boost_element_30d_price_sat: i64,
    pub paywall_boost_element_90d_price_sat: i64,
    pub paywall_boost_element_365d_price_sat: i64,
    pub lnbits_invoice_key: String,
    pub gitea_api_key: String,
    pub matrix_bot_password: String,
    pub lnd_invoices_macaroon: String,
    pub lnd_readonly_macaroon: String,
    pub ppq_key: String,
    pub xpub_spending: String,
    pub xpub_donations: String,
    pub xpub_treasury: String,
}

impl Conf {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::PaywallAddElementCommentPriceSat,
                Columns::PaywallBoostElement30DaysPriceSat,
                Columns::PaywallBoostElement90DaysPriceSat,
                Columns::PaywallBoostElement365DaysPriceSat,
                Columns::LNbitsInvoiceKey,
                Columns::GiteaApiKey,
                Columns::MatrixBotPassword,
                Columns::LndInvoicesMacaroon,
                Columns::LndReadonlyMacaroon,
                Columns::PpqKey,
                Columns::XpubSpending,
                Columns::XpubDonations,
                Columns::XpubTreasury,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            Ok(Self {
                paywall_add_element_comment_price_sat: row
                    .get(Columns::PaywallAddElementCommentPriceSat.as_str())?,
                paywall_boost_element_30d_price_sat: row
                    .get(Columns::PaywallBoostElement30DaysPriceSat.as_str())?,
                paywall_boost_element_90d_price_sat: row
                    .get(Columns::PaywallBoostElement90DaysPriceSat.as_str())?,
                paywall_boost_element_365d_price_sat: row
                    .get(Columns::PaywallBoostElement365DaysPriceSat.as_str())?,
                lnbits_invoice_key: row.get(Columns::LNbitsInvoiceKey.as_str())?,
                gitea_api_key: row.get(Columns::GiteaApiKey.as_str())?,
                matrix_bot_password: row.get(Columns::MatrixBotPassword.as_str())?,
                lnd_invoices_macaroon: row.get(Columns::LndInvoicesMacaroon.as_str())?,
                lnd_readonly_macaroon: row.get(Columns::LndReadonlyMacaroon.as_str())?,
                ppq_key: row.get(Columns::PpqKey.as_str())?,
                xpub_spending: row.get(Columns::XpubSpending.as_str())?,
                xpub_donations: row.get(Columns::XpubDonations.as_str())?,
                xpub_treasury: row.get(Columns::XpubTreasury.as_str())?,
            })
        }
    }
}
