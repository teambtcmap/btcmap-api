CREATE TABLE conf(
    id INTEGER PRIMARY KEY NOT NULL,
    paywall_add_element_comment_price_sat TEXT NOT NULL,
    paywall_boost_element_30d_price_sat TEXT NOT NULL,
    paywall_boost_element_90d_price_sat TEXT NOT NULL,
    paywall_boost_element_365d_price_sat TEXT NOT NULL
) STRICT;

INSERT INTO conf (paywall_add_element_comment_price_sat, paywall_boost_element_30d_price_sat, paywall_boost_element_90d_price_sat, paywall_boost_element_365d_price_sat) VALUES (500, 5000, 10000, 30000);
