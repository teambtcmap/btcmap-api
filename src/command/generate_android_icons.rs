use crate::model::element;
use crate::model::Element;
use crate::Connection;
use crate::Result;
use rusqlite::named_params;
use serde_json::Value;

pub async fn run(db: Connection) -> Result<()> {
    log::info!("Generating Android icons");

    let elements: Vec<Element> = db
        .prepare(element::SELECT_ALL)?
        .query_map(
            named_params! { ":limit": std::i32::MAX },
            element::SELECT_ALL_MAPPER,
        )?
        .collect::<Result<Vec<Element>, _>>()?
        .into_iter()
        .filter(|it| it.deleted_at.len() == 0)
        .collect();

    log::info!("Found {} elements", elements.len());

    let mut known = 0;
    let mut unknown = 0;

    for element in elements {
        let old_icon = element.tags["icon:android"].as_str().unwrap_or("");
        let new_icon = element.android_icon();

        if old_icon != new_icon {
            log::info!(
                "Updating icon for element {} ({old_icon} -> {new_icon})",
                element.id,
            );

            db.execute(
                element::INSERT_TAG,
                named_params! {
                    ":element_id": element.id,
                    ":tag_name": "$.icon:android",
                    ":tag_value": new_icon,
                },
            )?;
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        if new_icon == "question_mark" {
            unknown += 1;
        } else {
            known += 1;
        }
    }

    log::info!(
        "Finished generating Android icons. Known: {known}, unknown: {unknown}, coverage: {:.2}%",
        known as f64 / (known as f64 + unknown as f64) * 100.0
    );

    Ok(())
}

impl Element {
    pub fn android_icon(&self) -> String {
        let tags: &Value = &self.osm_json["tags"];

        let amenity = tags["amenity"].as_str().unwrap_or("");
        let cuisine = tags["cuisine"].as_str().unwrap_or("");
        let tourism = tags["tourism"].as_str().unwrap_or("");
        let shop = tags["shop"].as_str().unwrap_or("");
        let office = tags["office"].as_str().unwrap_or("");
        let leisure = tags["leisure"].as_str().unwrap_or("");
        let healthcare = tags["healthcare"].as_str().unwrap_or("");
        let building = tags["building"].as_str().unwrap_or("");
        let sport = tags["sport"].as_str().unwrap_or("");
        let craft = tags["craft"].as_str().unwrap_or("");
        let company = tags["company"].as_str().unwrap_or("");
        let telecom = tags["telecom"].as_str().unwrap_or("");
        let school = tags["school"].as_str().unwrap_or("");
        let place = tags["place"].as_str().unwrap_or("");
        let landuse = tags["landuse"].as_str().unwrap_or("");
        let club = tags["club"].as_str().unwrap_or("");
        let playground = tags["playground"].as_str().unwrap_or("");
        let industrial = tags["industrial"].as_str().unwrap_or("");
        let historic = tags["historic"].as_str().unwrap_or("");

        let mut icon_id: &str = "question_mark";

        if landuse == "retail" {
            icon_id = "storefront"
        }

        if building == "commercial" {
            icon_id = "business"
        }

        if building == "office" {
            icon_id = "business"
        }

        if building == "retail" {
            icon_id = "storefront"
        }

        if building == "church" {
            icon_id = "church"
        }

        if building == "school" {
            icon_id = "school"
        }

        if building == "industrial" {
            icon_id = "factory"
        }

        if office != "" {
            icon_id = "business"
        }

        if office == "company" {
            icon_id = "business"
        }

        if office == "it" {
            icon_id = "computer"
        }

        if office == "lawyer" {
            icon_id = "balance"
        }

        if office == "accountant" {
            icon_id = "attach_money"
        }

        if office == "architect" {
            icon_id = "architecture"
        }

        if office == "educational_institution" {
            icon_id = "school"
        }

        if office == "advertising_agency" {
            icon_id = "business"
        }

        if office == "estate_agent" {
            icon_id = "home"
        }

        if office == "therapist" {
            icon_id = "medical_services"
        }

        if office == "coworking" {
            icon_id = "group"
        }

        if office == "physician" {
            icon_id = "medical_services"
        }

        if office == "marketing" {
            icon_id = "business"
        }

        if office == "surveyor" {
            icon_id = "business"
        }

        if office == "financial" {
            icon_id = "attach_money"
        }

        if office == "association" {
            icon_id = "group"
        }

        if office == "engineer" {
            icon_id = "engineering"
        }

        if office == "telecommunication" {
            icon_id = "cell_tower"
        }

        if office == "coworking_space" {
            icon_id = "group"
        }

        if office == "construction" {
            icon_id = "engineering"
        }

        if office == "tax_advisor" {
            icon_id = "attach_money"
        }

        if office == "construction_company" {
            icon_id = "engineering"
        }

        if office == "travel_agent" {
            icon_id = "tour"
        }

        if office == "insurance" {
            icon_id = "business"
        }

        if office == "ngo" {
            icon_id = "business"
        }

        if office == "newspaper" {
            icon_id = "newspaper"
        }

        if office == "trade" {
            icon_id = "business"
        }

        if office == "private" {
            icon_id = "business"
        }

        if office == "guide" {
            icon_id = "tour"
        }

        if office == "foundation" {
            icon_id = "business"
        }

        if office == "web_design" {
            icon_id = "design_services"
        }

        if office == "graphic_design" {
            icon_id = "design_services"
        }

        if office == "limousine_service" {
            icon_id = "local_taxi"
        }

        if tourism == "hotel" {
            icon_id = "hotel";
        }

        if tourism == "attraction" {
            icon_id = "tour";
        }

        if tourism == "guest_house" {
            icon_id = "hotel";
        }

        if tourism == "apartment" {
            icon_id = "hotel";
        }

        if tourism == "hostel" {
            icon_id = "hotel";
        }

        if tourism == "chalet" {
            icon_id = "chalet";
        }

        if tourism == "camp_site" {
            icon_id = "camping";
        }

        if tourism == "gallery" {
            icon_id = "palette";
        }

        if tourism == "artwork" {
            icon_id = "palette";
        }

        if tourism == "information" {
            icon_id = "info_outline";
        }

        if tourism == "museum" {
            icon_id = "museum";
        }

        if tourism == "motel" {
            icon_id = "hotel";
        }

        if tourism == "spa" {
            icon_id = "spa";
        }

        if tourism == "theme_park" {
            icon_id = "attractions";
        }

        if tourism == "alpine_hut" {
            icon_id = "cottage";
        }

        if shop != "" {
            icon_id = "storefront";
        }

        if shop == "computer" {
            icon_id = "computer";
        }

        if shop == "clothes" {
            icon_id = "storefront";
        }

        if shop == "jewelry" {
            icon_id = "diamond";
        }

        if shop == "hairdresser" {
            icon_id = "content_cut";
        }

        if shop == "electronics" {
            icon_id = "computer";
        }

        if shop == "supermarket" {
            icon_id = "local_grocery_store";
        }

        if shop == "car_repair" {
            icon_id = "car_repair";
        }

        if shop == "beauty" {
            icon_id = "spa";
        }

        if shop == "books" {
            icon_id = "menu_book";
        }

        if shop == "furniture" {
            icon_id = "chair";
        }

        if shop == "convenience" {
            icon_id = "local_grocery_store";
        }

        if shop == "gift" {
            icon_id = "card_giftcard";
        }

        if shop == "travel_agency" {
            icon_id = "luggage";
        }

        if shop == "mobile_phone" {
            icon_id = "smartphone";
        }

        if shop == "tobacco" {
            icon_id = "smoking_rooms";
        }

        if shop == "car" {
            icon_id = "directions_car";
        }

        if shop == "bakery" {
            icon_id = "bakery_dining";
        }

        if shop == "massage" {
            icon_id = "spa";
        }

        if shop == "florist" {
            icon_id = "local_florist";
        }

        if shop == "bicycle" {
            icon_id = "pedal_bike";
        }

        if shop == "bicycle" {
            icon_id = "pedal_bike";
        }

        if shop == "e-cigarette" {
            icon_id = "vaping_rooms";
        }

        if shop == "optician" {
            icon_id = "visibility";
        }

        if shop == "photo" {
            icon_id = "photo_camera";
        }

        if shop == "deli" {
            icon_id = "tapas";
        }

        if shop == "sports" {
            icon_id = "sports";
        }

        if shop == "farm" {
            icon_id = "storefront";
        }

        if shop == "art" {
            icon_id = "palette";
        }

        if shop == "music" {
            icon_id = "music_note";
        }

        if shop == "hardware" {
            icon_id = "hardware";
        }

        if shop == "copyshop" {
            icon_id = "local_printshop";
        }

        if shop == "wine" {
            icon_id = "wine_bar";
        }

        if shop == "shoes" {
            icon_id = "storefront";
        }

        if shop == "alcohol" {
            icon_id = "liquor";
        }

        if shop == "toys" {
            icon_id = "toys";
        }

        if shop == "greengrocer" {
            icon_id = "storefront";
        }

        if shop == "car_parts" {
            icon_id = "directions_car";
        }

        if shop == "tatoo" {
            icon_id = "storefront";
        }

        if shop == "pawnbroker" {
            icon_id = "attach_money";
        }

        if shop == "garden_centre" {
            icon_id = "local_florist";
        }

        if shop == "butcher" {
            icon_id = "storefront";
        }

        if shop == "variety_store" {
            icon_id = "storefront";
        }

        if shop == "printing" {
            icon_id = "local_printshop";
        }

        if shop == "laundry" {
            icon_id = "local_laundry_service";
        }

        if shop == "kiosk" {
            icon_id = "storefront";
        }

        if shop == "pet" {
            icon_id = "pets";
        }

        if shop == "cannabis" {
            icon_id = "grass";
        }

        if shop == "boutique" {
            icon_id = "storefront";
        }

        if shop == "stationery" {
            icon_id = "edit";
        }

        if shop == "pastry" {
            icon_id = "bakery_dining";
        }

        if shop == "mall" {
            icon_id = "local_mall";
        }

        if shop == "hifi" {
            icon_id = "music_note";
        }

        if shop == "estate_agent" {
            icon_id = "home";
        }

        if shop == "cosmetics" {
            icon_id = "spa";
        }

        if shop == "coffee" {
            icon_id = "coffee";
        }

        if shop == "erotic" {
            icon_id = "adult_content";
        }

        if shop == "confectionery" {
            icon_id = "cake";
        }

        if shop == "beverages" {
            icon_id = "liquor";
        }

        if shop == "video_games" {
            icon_id = "games";
        }

        if shop == "newsagent" {
            icon_id = "newspaper";
        }

        if shop == "interior_decoration" {
            icon_id = "design_services";
        }

        if shop == "electrical" {
            icon_id = "electrical_services";
        }

        if shop == "doityourself" {
            icon_id = "hardware";
        }

        if shop == "antiques" {
            icon_id = "storefront";
        }

        if shop == "watches" {
            icon_id = "watch";
        }

        if shop == "trade" {
            icon_id = "storefront";
        }

        if shop == "tea" {
            icon_id = "emoji_food_beverage";
        }

        if shop == "scuba_diving" {
            icon_id = "scuba_diving";
        }

        if shop == "musical_instrument" {
            icon_id = "music_note";
        }

        if shop == "dairy" {
            icon_id = "storefront";
        }

        if shop == "chocolate" {
            icon_id = "storefront";
        }

        if shop == "anime" {
            icon_id = "storefront";
        }

        if shop == "tyres" {
            icon_id = "trip_origin";
        }

        if shop == "second_hand" {
            icon_id = "storefront";
        }

        if shop == "perfumery" {
            icon_id = "storefront";
        }

        if shop == "nutrition_supplements" {
            icon_id = "storefront";
        }

        if shop == "motorcycle" {
            icon_id = "two_wheeler";
        }

        if shop == "lottery" {
            icon_id = "storefront";
        }

        if shop == "locksmith" {
            icon_id = "lock";
        }

        if shop == "games" {
            icon_id = "games";
        }

        if shop == "funeral_directors" {
            icon_id = "church";
        }

        if shop == "department_store" {
            icon_id = "local_mall";
        }

        if shop == "chemist" {
            icon_id = "science";
        }

        if shop == "carpet" {
            icon_id = "storefront";
        }

        if shop == "water_sports" {
            icon_id = "pool";
        }

        if shop == "water" {
            icon_id = "sports";
        }

        if shop == "video" {
            icon_id = "videocam";
        }

        if shop == "tailor" {
            icon_id = "checkroom";
        }

        if shop == "storage_rental" {
            icon_id = "warehouse";
        }

        if shop == "storage" {
            icon_id = "warehouse";
        }

        if shop == "outdoor" {
            icon_id = "outdoor_grill";
        }

        if shop == "houseware" {
            icon_id = "chair";
        }

        if shop == "herbalist" {
            icon_id = "local_florist";
        }

        if shop == "health_food" {
            icon_id = "local_florist";
        }

        if shop == "grocery" {
            icon_id = "local_grocery_store";
        }

        if shop == "food" {
            icon_id = "local_grocery_store";
        }

        if shop == "curtain" {
            icon_id = "storefront";
        }

        if shop == "boat" {
            icon_id = "sailing";
        }

        if shop == "wholesale" {
            icon_id = "local_grocery_store";
        }

        if shop == "surf" {
            icon_id = "surfing";
        }

        if amenity == "restaurant" {
            icon_id = "restaurant"
        }

        if amenity == "atm" {
            icon_id = "local_atm"
        }

        if amenity == "cafe" {
            icon_id = "local_cafe"
        }

        if amenity == "bar" {
            icon_id = "local_bar"
        }

        if amenity == "bureau_de_change" {
            icon_id = "currency_exchange"
        }

        if amenity == "place_of_worship" {
            icon_id = "church"
        }

        if amenity == "fast_food" {
            icon_id = "lunch_dining"
        }

        if amenity == "bank" {
            icon_id = "account_balance"
        }

        if amenity == "dentist" {
            icon_id = "medical_services"
        }

        if amenity == "pub" {
            icon_id = "sports_bar"
        }

        if amenity == "doctors" {
            icon_id = "medical_services"
        }

        if amenity == "pharmacy" {
            icon_id = "local_pharmacy"
        }

        if amenity == "clinic" {
            icon_id = "medical_services"
        }

        if amenity == "school" {
            icon_id = "school"
        }

        if amenity == "taxi" {
            icon_id = "local_taxi"
        }

        if amenity == "studio" {
            icon_id = "mic"
        }

        if amenity == "fuel" {
            icon_id = "local_gas_station"
        }

        if amenity == "car_rental" {
            icon_id = "directions_car"
        }

        if amenity == "arts_centre" {
            icon_id = "palette"
        }

        if amenity == "police" {
            icon_id = "local_police"
        }

        if amenity == "hospital" {
            icon_id = "local_hospital"
        }

        if amenity == "brothel" {
            icon_id = "adult_content"
        }

        if amenity == "veterinary" {
            icon_id = "pets"
        }

        if amenity == "university" {
            icon_id = "school"
        }

        if amenity == "college" {
            icon_id = "school"
        }

        if amenity == "car_wash" {
            icon_id = "local_car_wash"
        }

        if amenity == "nightclub" {
            icon_id = "nightlife"
        }

        if amenity == "driving_school" {
            icon_id = "directions_car"
        }

        if amenity == "boat_rental" {
            icon_id = "directions_boat"
        }

        if amenity == "vending_machine" {
            icon_id = "storefront"
        }

        if amenity == "money_transfer" {
            icon_id = "currency_exchange"
        }

        if amenity == "marketplace" {
            icon_id = "storefront"
        }

        if amenity == "ice_cream" {
            icon_id = "icecream"
        }

        if amenity == "coworking_space" {
            icon_id = "business"
        }

        if amenity == "community_centre" {
            icon_id = "group"
        }

        if amenity == "kindergarten" {
            icon_id = "child_care"
        }

        if amenity == "internet_cafe" {
            icon_id = "public"
        }

        if amenity == "recycling" {
            icon_id = "delete"
        }

        if amenity == "payment_centre" {
            icon_id = "currency_exchange"
        }

        if amenity == "cinema" {
            icon_id = "local_movies"
        }

        if amenity == "childcare" {
            icon_id = "child_care"
        }

        if amenity == "bicycle_rental" {
            icon_id = "pedal_bike"
        }

        if amenity == "townhall" {
            icon_id = "group"
        }

        if amenity == "theatre" {
            icon_id = "account_balance"
        }

        if amenity == "post_office" {
            icon_id = "local_post_office"
        }

        if amenity == "payment_terminal" {
            icon_id = "currency_exchange"
        }

        if amenity == "office" {
            icon_id = "business"
        }

        if amenity == "language_school" {
            icon_id = "school"
        }

        if amenity == "charging_station" {
            icon_id = "electrical_services"
        }

        if amenity == "stripclub" {
            icon_id = "adult_content"
        }

        if amenity == "spa" {
            icon_id = "spa"
        }

        if amenity == "training" {
            icon_id = "school"
        }

        if amenity == "flight_school" {
            icon_id = "flight_takeoff"
        }

        if amenity == "motorcycle_rental" {
            icon_id = "two_wheeler"
        }

        if amenity == "dojo" {
            icon_id = "sports_martial_arts"
        }

        if amenity == "animal_breeding" {
            icon_id = "cruelty_free"
        }

        if amenity == "animal_shelter" {
            icon_id = "pets"
        }

        if amenity == "food_court" {
            icon_id = "restaurant"
        }

        if amenity == "dive_centre" {
            icon_id = "scuba_diving"
        }

        if leisure == "sports_centre" {
            icon_id = "fitness_center"
        }

        if leisure == "hackerspace" {
            icon_id = "computer"
        }

        if leisure == "fitness_centre" {
            icon_id = "fitness_center"
        }

        if leisure == "pitch" {
            icon_id = "sports"
        }

        if leisure == "resort" {
            icon_id = "beach_access"
        }

        if leisure == "park" {
            icon_id = "park"
        }

        if leisure == "beach_resort" {
            icon_id = "beach_access"
        }

        if leisure == "marina" {
            icon_id = "directions_boat"
        }

        if leisure == "golf_course" {
            icon_id = "golf_course"
        }

        if leisure == "garden" {
            icon_id = "local_florist"
        }

        if leisure == "escape_game" {
            icon_id = "games"
        }

        if leisure == "dance" {
            icon_id = "nightlife"
        }

        if leisure == "kayak_dock" {
            icon_id = "kayaking"
        }

        if leisure == "water_park" {
            icon_id = "pool"
        }

        if leisure == "horse_riding" {
            icon_id = "bedroom_baby"
        }

        if leisure == "adventure_park" {
            icon_id = "nature_people"
        }

        if leisure == "casino" {
            icon_id = "casino"
        }

        if leisure == "amusement_arcade" {
            icon_id = "videogame_asset"
        }

        if healthcare != "" {
            icon_id = "medical_services"
        }

        if healthcare == "dentist" {
            icon_id = "medical_services"
        }

        if healthcare == "doctor" {
            icon_id = "medical_services"
        }

        if healthcare == "clinic" {
            icon_id = "medical_services"
        }

        if healthcare == "pharmacy" {
            icon_id = "local_pharmacy"
        }

        if healthcare == "optometrist" {
            icon_id = "visibility"
        }

        if sport == "scuba_diving" {
            icon_id = "scuba_diving"
        }

        if sport == "soccer" {
            icon_id = "sports_soccer"
        }

        if craft == "yes" {
            icon_id = "construction"
        }

        if craft == "blacksmith" {
            icon_id = "hardware"
        }

        if craft == "photographer" {
            icon_id = "photo_camera"
        }

        if craft == "hvac" {
            icon_id = "hvac"
        }

        if craft == "signmaker" {
            icon_id = "hardware"
        }

        if craft == "brewery" {
            icon_id = "sports_bar"
        }

        if craft == "confectionery" {
            icon_id = "cake";
        }

        if craft == "tiler" {
            icon_id = "grid_view";
        }

        if craft == "painter" {
            icon_id = "imagesearch_roller";
        }

        if craft == "gardener" {
            icon_id = "grass";
        }

        if craft == "metal_construction" {
            icon_id = "construction";
        }

        if craft == "carpenter" {
            icon_id = "carpenter";
        }

        if craft == "joiner" {
            icon_id = "carpenter";
        }

        if craft == "cleaning" {
            icon_id = "cleaning_services";
        }

        if craft == "electrician" {
            icon_id = "electric_bolt";
        }

        if craft == "cabinet_maker" {
            icon_id = "chair";
        }

        if craft == "jeweller" {
            icon_id = "diamond";
        }

        if craft == "winery" {
            icon_id = "wine_bar";
        }

        if craft == "electronics_repair" {
            icon_id = "build";
        }

        if craft == "caterer" {
            icon_id = "cooking";
        }

        if craft == "agricultural_engines" {
            icon_id = "agriculture";
        }

        if craft == "roofer" {
            icon_id = "roofing";
        }

        if craft == "art" {
            icon_id = "palette";
        }

        if craft == "glaziery" {
            icon_id = "window";
        }

        if craft == "beekeeper" {
            icon_id = "hive"
        }

        if craft == "handicraft" {
            icon_id = "volunteer_activism"
        }

        if company == "transport" {
            icon_id = "directions_car"
        }

        if cuisine == "burger" {
            icon_id = "lunch_dining"
        }

        if cuisine == "pizza" {
            icon_id = "local_pizza"
        }

        if telecom == "data_center" {
            icon_id = "dns"
        }

        if place == "farm" {
            icon_id = "agriculture"
        }

        if school == "music" {
            icon_id = "music_note"
        }

        if club == "yes" {
            icon_id = "groups"
        }

        if club == "tech" {
            icon_id = "lan"
        }

        if playground == "structure" {
            icon_id = "attractions"
        }

        if industrial == "slaughterhouse" {
            icon_id = "surgical"
        }

        if historic == "castle" {
            icon_id = "castle"
        }

        if amenity == "fast_food" && cuisine == "ice_cream" {
            icon_id = "icecream";
        }

        if craft == "electronics_repair" && shop == "mobile_phone" {
            icon_id = "smartphone";
        }

        if craft == "electronics_repair" && shop == "computer" {
            icon_id = "computer";
        }

        icon_id.to_string()
    }
}
