ALTER TABLE conf ADD COLUMN boost_element_prices TEXT NOT NULL DEFAULT '[]';
UPDATE conf SET boost_element_prices = json_array(
    json_object('days', 30, 'sats', paywall_boost_element_30d_price_sat),
    json_object('days', 90, 'sats', paywall_boost_element_90d_price_sat),
    json_object('days', 365, 'sats', paywall_boost_element_365d_price_sat)
);
ALTER TABLE conf DROP COLUMN paywall_boost_element_30d_price_sat;
ALTER TABLE conf DROP COLUMN paywall_boost_element_90d_price_sat;
ALTER TABLE conf DROP COLUMN paywall_boost_element_365d_price_sat;
