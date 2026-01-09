use rusqlite::Row;
use std::sync::OnceLock;

pub const TABLE_NAME: &str = "conf";

pub enum Columns {
    PaywallAddElementCommentPriceSat,
    PaywallBoostElement30DaysPriceSat,
    PaywallBoostElement90DaysPriceSat,
    PaywallBoostElement365DaysPriceSat,
    LNbitsInvoiceKey,
    DiscordWebhookOsmChanges,
    DiscordWebhookApi,
    GiteaApiKey,
    MatrixBotPassword,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::PaywallAddElementCommentPriceSat => "paywall_add_element_comment_price_sat",
            Columns::PaywallBoostElement30DaysPriceSat => "paywall_boost_element_30d_price_sat",
            Columns::PaywallBoostElement90DaysPriceSat => "paywall_boost_element_90d_price_sat",
            Columns::PaywallBoostElement365DaysPriceSat => "paywall_boost_element_365d_price_sat",
            Columns::LNbitsInvoiceKey => "lnbits_invoice_key",
            Columns::DiscordWebhookOsmChanges => "discord_webhook_osm_changes",
            Columns::DiscordWebhookApi => "discord_webhook_api",
            Columns::GiteaApiKey => "gitea_api_key",
            Columns::MatrixBotPassword => "matrix_bot_password",
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
    pub discord_webhook_osm_changes: String,
    pub discord_webhook_api: String,
    pub gitea_api_key: String,
    pub matrix_bot_password: String,
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
                Columns::DiscordWebhookOsmChanges,
                Columns::DiscordWebhookApi,
                Columns::GiteaApiKey,
                Columns::MatrixBotPassword,
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
                discord_webhook_osm_changes: row.get(Columns::DiscordWebhookOsmChanges.as_str())?,
                discord_webhook_api: row.get(Columns::DiscordWebhookApi.as_str())?,
                gitea_api_key: row.get(Columns::GiteaApiKey.as_str())?,
                matrix_bot_password: row.get(Columns::MatrixBotPassword.as_str())?,
            })
        }
    }

    #[cfg(test)]
    pub fn mock() -> Conf {
        Conf {
            paywall_add_element_comment_price_sat: 0,
            paywall_boost_element_30d_price_sat: 0,
            paywall_boost_element_90d_price_sat: 0,
            paywall_boost_element_365d_price_sat: 0,
            lnbits_invoice_key: "".to_string(),
            discord_webhook_osm_changes: "".to_string(),
            discord_webhook_api: "".to_string(),
            gitea_api_key: "".to_string(),
            matrix_bot_password: "".to_string(),
        }
    }
}
