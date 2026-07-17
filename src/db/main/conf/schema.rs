use rusqlite::Row;
use serde::Deserialize;
use serde::Serialize;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "conf";

#[allow(non_camel_case_types)]
#[derive(strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Columns {
    PaywallAddElementCommentPriceSat,
    BoostElementPrices,
    LnbitsInvoiceKey,
    GiteaApiKey,
    MatrixBotPassword,
    LndInvoicesMacaroon,
    LndReadonlyMacaroon,
    PpqKey,
    XpubSpending,
    XpubDonations,
    XpubTreasury,
    ElectrumUrl,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BoostPrice {
    pub days: i64,
    pub sats: i64,
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct Conf {
    pub paywall_add_element_comment_price_sat: i64,
    pub boost_element_prices: Vec<BoostPrice>,
    pub lnbits_invoice_key: String,
    pub gitea_api_key: String,
    pub matrix_bot_password: String,
    pub lnd_invoices_macaroon: String,
    pub lnd_readonly_macaroon: String,
    pub ppq_key: String,
    pub xpub_spending: String,
    pub xpub_donations: String,
    pub xpub_treasury: String,
    pub electrum_url: String,
}

impl Conf {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::PaywallAddElementCommentPriceSat,
                Columns::BoostElementPrices,
                Columns::LnbitsInvoiceKey,
                Columns::GiteaApiKey,
                Columns::MatrixBotPassword,
                Columns::LndInvoicesMacaroon,
                Columns::LndReadonlyMacaroon,
                Columns::PpqKey,
                Columns::XpubSpending,
                Columns::XpubDonations,
                Columns::XpubTreasury,
                Columns::ElectrumUrl,
            ]
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            let boost_element_prices: String = row.get(Columns::BoostElementPrices.as_ref())?;
            let boost_element_prices =
                serde_json::from_str(&boost_element_prices).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            Ok(Self {
                paywall_add_element_comment_price_sat: row
                    .get(Columns::PaywallAddElementCommentPriceSat.as_ref())?,
                boost_element_prices,
                lnbits_invoice_key: row.get(Columns::LnbitsInvoiceKey.as_ref())?,
                gitea_api_key: row.get(Columns::GiteaApiKey.as_ref())?,
                matrix_bot_password: row.get(Columns::MatrixBotPassword.as_ref())?,
                lnd_invoices_macaroon: row.get(Columns::LndInvoicesMacaroon.as_ref())?,
                lnd_readonly_macaroon: row.get(Columns::LndReadonlyMacaroon.as_ref())?,
                ppq_key: row.get(Columns::PpqKey.as_ref())?,
                xpub_spending: row.get(Columns::XpubSpending.as_ref())?,
                xpub_donations: row.get(Columns::XpubDonations.as_ref())?,
                xpub_treasury: row.get(Columns::XpubTreasury.as_ref())?,
                electrum_url: row.get(Columns::ElectrumUrl.as_ref())?,
            })
        }
    }
}
