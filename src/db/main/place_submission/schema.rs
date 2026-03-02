use rusqlite::Row;
use serde_json::{Map, Value};
use std::sync::OnceLock;
use time::OffsetDateTime;
use url::Url;

pub const TABLE_NAME: &str = "place_submission";

pub enum Columns {
    Id,
    Origin,
    ExternalId,
    Lat,
    Lon,
    Category,
    Name,
    ExtraFields,
    TicketUrl,
    Revoked,
    CreatedAt,
    UpdatedAt,
    ClosedAt,
    DeletedAt,
}

impl Columns {
    pub fn as_str(&self) -> &'static str {
        match self {
            Columns::Id => "id",
            Columns::Origin => "origin",
            Columns::ExternalId => "external_id",
            Columns::Lat => "lat",
            Columns::Lon => "lon",
            Columns::Category => "category",
            Columns::Name => "name",
            Columns::ExtraFields => "extra_fields",
            Columns::TicketUrl => "ticket_url",
            Columns::Revoked => "revoked",
            Columns::CreatedAt => "created_at",
            Columns::UpdatedAt => "updated_at",
            Columns::ClosedAt => "closed_at",
            Columns::DeletedAt => "deleted_at",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlaceSubmission {
    pub id: i64,
    pub origin: String,
    pub external_id: String,
    pub lat: f64,
    pub lon: f64,
    pub category: String,
    pub name: String,
    pub extra_fields: Map<String, Value>,
    pub ticket_url: Option<String>,
    pub revoked: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
    pub closed_at: Option<OffsetDateTime>,
    pub deleted_at: Option<OffsetDateTime>,
}

impl PlaceSubmission {
    pub fn projection() -> &'static str {
        static PROJECTION: OnceLock<String> = OnceLock::new();
        PROJECTION.get_or_init(|| {
            [
                Columns::Id,
                Columns::Origin,
                Columns::ExternalId,
                Columns::Lat,
                Columns::Lon,
                Columns::Category,
                Columns::Name,
                Columns::ExtraFields,
                Columns::TicketUrl,
                Columns::Revoked,
                Columns::CreatedAt,
                Columns::UpdatedAt,
                Columns::ClosedAt,
                Columns::DeletedAt,
            ]
            .iter()
            .map(Columns::as_str)
            .collect::<Vec<_>>()
            .join(", ")
        })
    }

    pub const fn mapper() -> fn(&Row) -> rusqlite::Result<Self> {
        |row| {
            let extra_fields: String = row.get(Columns::ExtraFields.as_str())?;
            let extra_fields = serde_json::from_str(&extra_fields).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    2,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })?;

            Ok(Self {
                id: row.get(Columns::Id.as_str())?,
                origin: row.get(Columns::Origin.as_str())?,
                external_id: row.get(Columns::ExternalId.as_str())?,
                lat: row.get(Columns::Lat.as_str())?,
                lon: row.get(Columns::Lon.as_str())?,
                category: row.get(Columns::Category.as_str())?,
                name: row.get(Columns::Name.as_str())?,
                extra_fields,
                ticket_url: row.get(Columns::TicketUrl.as_str())?,
                revoked: row.get(Columns::Revoked.as_str())?,
                created_at: row.get(Columns::CreatedAt.as_str())?,
                updated_at: row.get(Columns::UpdatedAt.as_str())?,
                closed_at: row.get(Columns::ClosedAt.as_str())?,
                deleted_at: row.get(Columns::DeletedAt.as_str())?,
            })
        }
    }

    pub fn icon(&self) -> String {
        match self.origin.as_str() {
            "square" => match self.category.as_str() {
                "individual_use" => "person",
                "beauty_and_barber_shops" => "content_cut",
                "professional_services" => "work",
                "clothing_and_accessories" => "checkroom",
                "misc_retail" => "storefront",
                "music_and_entertainment" => "music_note",
                "consultant" => "emoji_people",
                "food_stores_convenience_stores_and_specialty_markets" => "storefront",
                "personal_services" => "person",
                "art_design_and_photography" => "brush",
                "contractors" => "engineering",
                "charitible_orgs" => "volunteer_activism",
                "food_truck_cart" => "lunch_dining",
                "medical_services_and_health_practitioners" => "medical_services",
                "taxicabs_and_limousines" => "local_taxi",
                "retail_shops" => "storefront",
                "outdoor_markets" => "storefront",
                "restaurants" => "restaurant",
                "jewelry_and_watches" => "diamond",
                "web_dev_design" => "computer",
                "education" => "school",
                "apparel_and_accessory_shops" => "apparel",
                "membership_organizations" => "groups",
                "bakery" => "bakery_dining",
                "cultural_attractions" => "museum",
                "catering" => "briefcase_meal",
                "automotive_services" => "car_repair",
                "furniture_home_goods" => "scene",
                "health_and_beauty_spas" => "spa",
                "cleaning" => "cleaning_services",
                "landscaping_and_horticultural_services" => "yard",
                "medical_practitioners" => "medical_services",
                "coffee_tea_shop" => "local_cafe",
                "hobby_shop" => "palette",
                "special_trade_contractors" => "engineering",
                "membership_clubs" => "groups",
                "accounting" => "account_balance",
                "delivery_moving_and_storage" => "local_shipping",
                "real_estate" => "real_estate_agent",
                "bar_club_lounge" => "local_bar",
                "recreation_services" => "sports_soccer",
                "books_mags_music_and_video" => "library_books",
                "legal_services" => "gavel",
                "electronics" => "devices",
                "repair_shops_and_related_services" => "build",
                "computer_equipment_software_maintenance_repair_services" => "computer",
                "heating_plumbing_and_air_conditioning" => "ac_unit",
                "religious_organization" => "church",
                "flowers_and_gifts" => "local_florist",
                "grocery_market" => "grocery",
                "sporting_goods" => "sports_soccer",
                "child_care" => "child_care",
                "electrical_services" => "electrical_services",
                "theatrical_arts" => "theater_comedy",
                "cleaning_services" => "cleaning_services",
                "pet_store" => "pets",
                "car_washes" => "local_car_wash",
                "business_services" => "work",
                "printing_services" => "print",
                "dentistry" => "dentistry",
                "dry_cleaning_and_laundry" => "local_laundry_service",
                "movies_film" => "movie",
                "esthetic_salon" => "self_care",
                "motor_vehicle_supplies" => "directions_car",
                "roofing_siding_and_sheet_metal_work_contractors" => "roofing",
                "hardware_store" => "hardware",
                "utilities" => "electrical_services",
                "political_organizations" => "volunteer_activism",
                "massage" => "massage",
                "sporting_events" => "sports",
                "tourism" => "travel_explore",
                "carpet_cleaning" => "cleaning_services",
                "beauty_parlors_and_barber_shops" => "content_cut",
                "cigar_stores_and_stands" => "smoking_rooms",
                "sports_facilities" => "sports_soccer",
                "other_education" => "school",
                "art_dealers_galleries" => "brush",
                "veterinary_services" => "pets",
                "clothing_shoe_repair_alterations" => "checkroom",
                "hotels_and_lodging" => "hotel",
                "office_supply" => "business_center",
                "direct_marketing_catalog_and_retail_merchant" => "storefront",
                "eyewear" => "visibility",
                "used_merchandise_and_secondhand_stores" => "storefront",
                "pest_control" => "bug_report",
                "nail_salon" => "health_and_beauty",
                "delivery_services" => "local_shipping",
                "optometrist_eye_care" => "visibility",
                "garden_supply_shop" => "yard",
                "photographer" => "camera_alt",
                "book_stores" => "library_books",
                "towing_services" => "local_shipping",
                "childrens_clothing_stores" => "child_care",
                "schools_and_educational_services" => "school",
                "antique_store" => "storefront",
                "package_stores_beer_wine_and_liquor" => "storefront",
                "transportation_services" => "transportation",
                "furniture_home_and_office_equipment" => "weekend",
                "travel_agencies_and_tour_operators" => "travel_explore",
                "direct_marketing_catalog_mail_order_internet_merchant" => "shopping_cart",
                "medical_and_dental_labs" => "science",
                "hair_removal" => "health_and_beauty",
                "womens_accessories" => "health_and_beauty",
                "architectural_and_surveying" => "architecture",
                "travel_tourism" => "travel_explore",
                "watch_jewelry_repair" => "diamond",
                "bail_bonds" => "gavel",
                "drug_stores_and_pharmacies" => "local_pharmacy",
                "prep_cram_schools" => "school",
                "antique_reproductions" => "storefront",
                "nursing_and_personal_care_facilities" => "health_cross",
                "automotive_body_repair_shops" => "car_repair",
                "language_schools" => "school",
                "amusement_parks" => "attractions",
                "dance_schools" => "school",
                "funeral_service_and_crematories" => "deceased",
                "misc_general_merchandise" => "storefront",
                "advertising_services" => "campaign",
                "medical_equipment_and_supplies" => "biotech",
                "misc_home_furnishing" => "weekend",
                "marriage_consultancy" => "favorite",
                "automotive_parts_accessories_stores" => "car_repair",
                "dairy_product_stores" => "icecream",
                "personal_computer_school" => "computer",
                "freezer_and_locker_meat_provisioners" => "storefront",
                "gift_shop" => "card_giftcard",
                "womens_apparel" => "checkroom",
                "cosmetic_stores" => "health_and_beauty",
                "electronics_repair_shops" => "devices",
                "automotive_tire_stores" => "car_repair",
                "sporting_and_recreational_camps" => "sports_soccer",
                "tailors_and_alterations" => "checkroom",
                "candy_nut_and_confectionery_stores" => "cookie",
                "car_and_truck_dealers" => "directions_car",
                "artists_supply_and_craft_shops" => "palette",
                "parking_lots_and_garages" => "local_parking",
                "hardware_equipment_and_supplies" => "build",
                "used_automobile_dealers" => "directions_car",
                "swimming_pools_sales_service" => "pool",
                "bicycle_shops" => "directions_bike",
                "tutoring" => "school",
                "rv_parks_and_campgrounds" => "camping",
                "household_appliance_store" => "kitchen",
                "marinas_service_and_supplies" => "directions_boat",
                "protective_security_services" => "security",
                "service_stations" => "local_gas_station",
                "public_warehousing_and_storage" => "warehouse",
                "florist_supplies" => "local_florist",
                "family_apparel" => "apparel",
                "sewing_stores" => "checkroom",
                "furniture_repair_and_refinishing" => "weekend",
                "concrete_work_contractors" => "engineering",
                "ticket_sales" => "confirmation_number",
                "agricultural_cooperatives" => "agriculture",
                "floor_covering_stores" => "hardware",
                "misc_publishing_and_printing" => "print",
                "computers_peripheral_equipment_and_software" => "computer",
                "home_supply_warehouse_stores" => "construction",
                "misc_automotive_dealers" => "directions_car",
                "record_shops" => "music_note",
                "automotive_paint_shops" => "directions_car",
                "wholesale_books_periodicals_and_newspapers" => "library_books",
                "sports_stores" => "sports_soccer",
                "plumbing_heating_equipment" => "plumbing",
                "music_instruments_and_sheet_music" => "music_note",
                "misc_nondurable_goods" => "storefront",
                "durable_goods" => "storefront",
                "religious_goods_stores" => "church",
                "misc_commercial_equipment" => "storefront",
                "tire_retreading_and_repair_shops" => "car_repair",
                "wig_and_toupee_stores" => "face",
                "carpentry_contractors" => "carpenter",
                "bus_lines" => "directions_bus",
                "construction_materials" => "construction",
                "drapery_window_covering_and_upholstery" => "window",
                "truck_and_utility_trailer_rentals" => "local_shipping",
                "tool_furniture_rental" => "build",
                "shoe_stores" => "storefront",
                "lumber_and_building_materials_stores" => "construction",
                "mens_apparel_and_accessory_shops" => "apparel",
                "industrial_supplies" => "storefront",
                "welding_repair" => "engineering",
                "telecom_equipment" => "settings_input_antenna",
                "metal_service_centers" => "engineering",
                "electrical_parts_and_equipment" => "electrical_services",
                "insulation_stonework_contractors" => "engineering",
                "motor_home_recreational_vehicle_rentals" => "hotel",
                "cable_and_pay_television" => "tv",
                "motion_pictures_and_video_production_distribution" => "movie",
                "luggage_and_leather_goods_stores" => "luggage",
                "fuel_oil_liquefied_petroleum" => "local_gas_station",
                "uniforms_and_commercial_clothing" => "checkroom",
                "photo_developing" => "camera",
                "glassware_crystal_stores" => "storefront",
                "fabric_wholesale" => "storefront",
                "small_appliance_repair" => "kitchen",
                "motorcycle_dealers" => "two_wheeler",
                "airports_terminals_flying_fields" => "flight",
                "air_conditioning_repair" => "settings",
                "grocery" => "grocery",
                "insurance" => "verified_user",
                "hearing_aids_sales_service_stores" => "hearing",
                "pawn_shops" => "attach_money",
                "stenographic_and_secreterial_services" => "work",
                "nonmedical_testing_labs" => "science",
                "discount_stores" => "sell",
                "ambulance_services" => "medical_services",
                "video_game_arcades_establishments" => "sports_esports",
                "office_supplies" => "business_center",
                "glass_paint_and_wallpaper_stores" => "imagesearch_roller",
                "aquariums" => "waves",
                "golf_courses" => "golf_course",
                "typesetting_platemaking_services" => "print",
                "variety_stores" => "storefront",
                "news_dealers_and_newstands" => "library_books",
                "clothing_retail" => "checkroom",
                "tanning_salon" => "bath_bedrock",
                "petroleum_and_petroleum_products" => "local_gas_station",
                "wholesale_clubs" => "groups",
                "fireplace_stores" => "fireplace",
                "chemical_and_allied_products" => "science",
                "camera_and_photographic_supply_stores" => "photo_camera",
                "paints_varnishes_and_supplies" => "format_paint",
                "hospitals" => "local_hospital",
                "video_amusement_game_supplies" => "sports_esports",
                "orthepedic_goods_prosthetic_devices" => "accessibility",
                "office_and_commercial_furniture" => "desk",
                "office_photographic_copy_film_equipment" => "business_center",
                "bowling_alleys" => "sports",
                "wrecking_and_salvage_yards" => "construction",
                "mobile_home_dealers" => "rv_hookup",
                "commercial_footwear" => "checkroom",
                "commuter_transportation" => "commute",
                "department_stores" => "storefront",
                "financial_institution" => "account_balance",
                "duty_free_store" => "storefront",
                "movie_rental_stores" => "movie",
                "furriers_and_fur_shops" => "checkroom",
                "tent_and_awning_shops" => "camping",
                "charitable_social_service_organizations" => "volunteer_activism",
                "electric_razor_stores" => "store",
                "billiard_and_pool_establishments" => "sports",
                "typewriter_sales_rental_stores" => "keyboard",
                "passenger_railways" => "train",
                "ghost_kitchen" => "restaurant",
                "apparel" => "apparel",
                "garden_supply" => "yard",
                "cigar_stands" => "smoking_rooms",
                _ => "currency_bitcoin",
            },
            _ => "store",
        }
        .to_string()
    }

    pub fn description(&self) -> Option<String> {
        self.extra_fields
            .get("description")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn address(&self) -> Option<String> {
        self.extra_fields
            .get("address")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn opening_hours(&self) -> Option<String> {
        self.extra_fields
            .get("opening_hours")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn phone(&self) -> Option<String> {
        self.extra_fields
            .get("phone")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn website(&self) -> Option<String> {
        let key = "website";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn twitter(&self) -> Option<String> {
        let key = "twitter";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn facebook(&self) -> Option<String> {
        let key = "facebook";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn instagram(&self) -> Option<String> {
        let key = "instagram";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn line(&self) -> Option<String> {
        let key = "line";

        if self.extra_fields.contains_key(key) && self.extra_fields[key].is_string() {
            let result = self.extra_fields[key].as_str().unwrap_or("");

            return if !result.is_empty() && is_valid_url(result) {
                Some(result.to_string())
            } else {
                None
            };
        }

        None
    }

    pub fn email(&self) -> Option<String> {
        self.extra_fields
            .get("email")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn image(&self) -> Option<String> {
        self.extra_fields
            .get("icon_url")
            .map(|it| it.as_str().unwrap_or("").to_string())
    }

    pub fn payment_provider(&self) -> Option<String> {
        if self.origin == "square" {
            Some(self.origin.clone())
        } else {
            None
        }
    }
}

fn is_valid_url(url: &str) -> bool {
    match Url::parse(url) {
        Ok(url) => url.scheme() == "http" || url.scheme() == "https",
        Err(_) => false,
    }
}
